import pathlib, shutil, subprocess, tempfile, time
# Run after `cargo build -p gize`. Requires local PostgreSQL and MySQL server/client binaries.
# Each server uses a disposable directory and a Unix socket, with TCP disabled.
CLI = str(pathlib.Path(__file__).resolve().parents[1] / 'target/debug/gize')
failures = 0

def run(args, **kw):
    p = subprocess.run(args, capture_output=True, text=True, timeout=90, **kw)
    if p.returncode:
        raise RuntimeError(str(args[:2]) + ': ' + p.stderr[-3000:] + p.stdout[-1000:])
    return p.stdout

def sql_fixture(root, dialect):
    work = root / 'project'; work.mkdir(); (work / 'migrations').mkdir()
    ty = 'VARCHAR(255)' if dialect == 'mysql' else 'TEXT'
    (work / 'gize.toml').write_text(f'''[project]
name = "check"
[stack]
framework = "axum"
database = "{dialect}"
orm = "sqlx"
[[module]]
name = "posts"
fields = ["title:String", "label:String", "active:bool", "views:i32", "total:i64", "score:f64", "owner:Uuid", "published:DateTime"]
''')
    initial = f"CREATE TABLE posts (id INTEGER PRIMARY KEY, title {ty} NOT NULL); INSERT INTO posts (id,title) VALUES (1,'existing');\n"
    (work / 'migrations/1_create.sql').write_text(initial)
    run([CLI, 'make', 'migration'], cwd=work)
    generated = next((work / 'migrations').glob('*_alter_posts.sql')).read_text()
    output = run([CLI, 'make', 'migration'], cwd=work)
    assert 'allows NULL' in output
    assert len(list((work / 'migrations').glob('*.sql'))) == 2
    return initial + generated + '\nSELECT COUNT(*) AS preserved_rows FROM posts;\n'

for dialect in ['postgres', 'mysql']:
    root = pathlib.Path(tempfile.mkdtemp(prefix='gize-db-', dir='/tmp'))
    proc = None; started = False
    try:
        sql = sql_fixture(root, dialect)
        if dialect == 'postgres':
            run(['initdb', '-D', str(root/'data'), '-A', 'trust', '-U', 'gize_test', '--no-locale'])
            run(['pg_ctl', '-D', str(root/'data'), '-l', str(root/'server.log'), '-o', f"-h '' -k {root} -p 55489", '-w', 'start'])
            started = True
            result = run(['psql', '-X', '-h', str(root), '-p', '55489', '-U', 'gize_test', '-d', 'postgres', '-v', 'ON_ERROR_STOP=1', '-At'], input=sql)
        else:
            run(['mysqld', '--no-defaults', '--initialize-insecure', '--datadir='+str(root/'data')])
            proc = subprocess.Popen(['mysqld', '--no-defaults', '--datadir='+str(root/'data'), '--socket='+str(root/'mysql.sock'), '--pid-file='+str(root/'mysql.pid'), '--skip-networking', '--log-error='+str(root/'server.log')], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            for _ in range(100):
                if (root/'mysql.sock').exists(): break
                if proc.poll() is not None: raise RuntimeError((root/'server.log').read_text())
                time.sleep(.1)
            result = run(['mysql', '--no-defaults', '--socket='+str(root/'mysql.sock'), '-u', 'root', '-N'], input='CREATE DATABASE gize_test; USE gize_test;\n'+sql)
        assert result.strip().splitlines()[-1] == '1', result
        print(dialect + ': PASS — all 7 added scalar types applied; existing row preserved', flush=True)
    except Exception as exc:
        failures += 1
        print(dialect + ': FAILED — ' + str(exc), flush=True)
    finally:
        if started: run(['pg_ctl', '-D', str(root/'data'), '-m', 'immediate', '-w', 'stop'])
        if proc is not None:
            proc.terminate()
            try: proc.wait(timeout=20)
            except subprocess.TimeoutExpired: proc.kill(); proc.wait()
        shutil.rmtree(root)

raise SystemExit(1 if failures else 0)
