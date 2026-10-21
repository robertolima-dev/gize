import pathlib, re, shutil, sqlite3, subprocess, tempfile, time
# Checks the reserved-word lists in crates/gize-core/src/validation.rs against real servers.
# Requires local PostgreSQL and MySQL server/client binaries (same setup as
# check_migration_dialects.py). Every keyword known to any of the three databases is used as
# a column and as a table name in the unquoted SQL shapes Gize generates; a word that breaks
# that SQL but is missing from the Rust list fails the check. Words the Rust list adds for
# newer releases (MySQL 8.x, PostgreSQL 16+) pass locally and are only reported.
REPO = pathlib.Path(__file__).resolve().parents[1]
VALIDATION = (REPO / 'crates/gize-core/src/validation.rs').read_text()
IDENT = re.compile(r'^[a-z_][a-z0-9_]*$')

def rust_list(name):
    body = re.search(name + r': &\[&str\] = &\[(.*?)\];', VALIDATION, re.S).group(1)
    return set(re.findall(r'"([^"]+)"', body))

# SQLite exposes no keyword table through SQL; this is its documented keyword list.
SQLITE_KEYWORDS = '''abort action add after all alter always analyze and as asc attach
autoincrement before begin between by cascade case cast check collate column commit conflict
constraint create cross current current_date current_time current_timestamp database default
deferrable deferred delete desc detach distinct do drop each else end escape except exclude
exclusive exists explain fail filter first following for foreign from full generated glob group
groups having if ignore immediate in index indexed initially inner insert instead intersect into
is isnull join key last left like limit match materialized natural no not nothing notnull null
nulls of offset on or order others outer over partition plan pragma preceding primary query raise
range recursive references regexp reindex release rename replace restrict returning right
rollback row rows savepoint select set table temp temporary then ties to transaction trigger
unbounded union unique update using vacuum values view virtual when where window with
without'''.split()

def shapes(w, returning):
    ret = ' RETURNING *' if returning else ''
    return (f"CREATE TABLE t (id INTEGER PRIMARY KEY, {w} VARCHAR(20) NOT NULL);"
            f"INSERT INTO t (id, {w}) VALUES (1, 'x'){ret};"
            f"UPDATE t SET {w} = 'y' WHERE id = 1{ret};"
            f"SELECT * FROM t ORDER BY id;"
            f"CREATE TABLE u (id INTEGER PRIMARY KEY);"
            f"ALTER TABLE u ADD COLUMN {w} VARCHAR(20);"
            f"CREATE TABLE {w} (id INTEGER PRIMARY KEY, name VARCHAR(20) NOT NULL);"
            f"CREATE TABLE r (id INTEGER PRIMARY KEY, ref_id INTEGER, FOREIGN KEY (ref_id) REFERENCES {w}(id));"
            f"INSERT INTO {w} (id, name) VALUES (1, 'x'){ret};"
            f"UPDATE {w} SET name = 'y' WHERE id = 1{ret};"
            f"SELECT * FROM {w} WHERE id = 1 ORDER BY id;"
            f"DELETE FROM r; DELETE FROM {w} WHERE id = 1;"
            f"DROP TABLE r; DROP TABLE {w}; DROP TABLE u; DROP TABLE t;")

def sqlite_ok(w):
    try:
        sqlite3.connect(':memory:').executescript(shapes(w, True))
        return True
    except sqlite3.Error:
        return False

root = pathlib.Path(tempfile.mkdtemp(prefix='gize-kw-', dir='/tmp'))
proc = None; started = False; failures = 0
try:
    subprocess.run(['initdb', '-D', str(root/'pg'), '-A', 'trust', '-U', 'gize_test', '--no-locale'], capture_output=True, check=True)
    subprocess.run(['pg_ctl', '-D', str(root/'pg'), '-l', str(root/'pg.log'), '-o', f"-h '' -k {root} -p 55490", '-w', 'start'], capture_output=True, check=True)
    started = True
    psql = ['psql', '-X', '-h', str(root), '-p', '55490', '-U', 'gize_test', '-d', 'postgres', '-qAt', '-v', 'ON_ERROR_STOP=1', '-c']
    pg_words = subprocess.run(psql + ['SELECT word FROM pg_get_keywords()'], capture_output=True, text=True, check=True).stdout.split()

    subprocess.run(['mysqld', '--no-defaults', '--initialize-insecure', '--datadir='+str(root/'my')], capture_output=True, check=True)
    proc = subprocess.Popen(['mysqld', '--no-defaults', '--datadir='+str(root/'my'), '--socket='+str(root/'m.sock'), '--pid-file='+str(root/'m.pid'), '--skip-networking', '--log-error='+str(root/'my.log')], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(300):
        if (root/'m.sock').exists(): break
        if proc.poll() is not None: raise RuntimeError((root/'my.log').read_text())
        time.sleep(.1)
    mysql = ['mysql', '--no-defaults', '--socket='+str(root/'m.sock'), '-u', 'root']
    subprocess.run(mysql + ['-e', 'CREATE DATABASE gize_test'], capture_output=True, check=True)
    mysql += ['gize_test', '-e']

    candidates = sorted({w for w in pg_words + SQLITE_KEYWORDS + list(rust_list('MYSQL_RESERVED')) if IDENT.match(w)})
    broken = {'POSTGRES_RESERVED': set(), 'SQLITE_RESERVED': set(), 'MYSQL_RESERVED': set()}
    for w in candidates:
        if subprocess.run(psql + ['BEGIN;' + shapes(w, True) + 'ROLLBACK;'], capture_output=True).returncode:
            broken['POSTGRES_RESERVED'].add(w)
        if not sqlite_ok(w):
            broken['SQLITE_RESERVED'].add(w)
        subprocess.run(mysql + [f'DROP TABLE IF EXISTS r, t, u, `{w}`'], capture_output=True)
        if subprocess.run(mysql + [shapes(w, False)], capture_output=True).returncode:
            broken['MYSQL_RESERVED'].add(w)

    for name, words in broken.items():
        listed = rust_list(name)
        missing = sorted(words - listed)
        extra = sorted(listed - words)
        if missing:
            failures += 1
            print(f'{name}: FAILED — breaks generated SQL but is not listed: {" ".join(missing)}')
        else:
            print(f'{name}: PASS — {len(words)} breaking words, all listed')
        if extra:
            print(f'  listed for newer releases (accepted by the local server): {" ".join(extra)}')
finally:
    if started: subprocess.run(['pg_ctl', '-D', str(root/'pg'), '-m', 'immediate', '-w', 'stop'], capture_output=True)
    if proc is not None:
        proc.terminate()
        try: proc.wait(timeout=20)
        except subprocess.TimeoutExpired: proc.kill(); proc.wait()
    shutil.rmtree(root)

raise SystemExit(1 if failures else 0)
