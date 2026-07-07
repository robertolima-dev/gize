import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, labelOf, titleCase } from "./api";
import { Icon } from "./icons";
import { toast } from "./toast";
import type { FieldDesc, ResourceDesc } from "./resources";

type Row = Record<string, any>;
const PAGE_SIZE = 25;

function shortId(id: unknown): string {
  const s = String(id ?? "");
  return s.length > 10 ? s.slice(0, 8) + "…" : s;
}
function fmtDate(v: unknown): string {
  if (typeof v !== "string") return "";
  const d = new Date(v);
  return isNaN(d.getTime())
    ? v
    : d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

/** Resolves a foreign-key id to a readable label from the related resource (cached). */
function RefCell({ resource, id }: { resource: string; id: string }) {
  const { data } = useQuery<Row[]>({ queryKey: [resource], queryFn: () => api.list(resource) });
  const row = (data ?? []).find((r) => r.id === id);
  return row ? <span>{labelOf(row)}</span> : <span className="id">{shortId(id)}</span>;
}

function Cell({ field, value }: { field: FieldDesc; value: unknown }) {
  if (field.kind === "boolean") {
    return value ? (
      <span className="pill ok">
        <span className="dot" />
        Yes
      </span>
    ) : (
      <span className="pill off">
        <span className="dot" />
        No
      </span>
    );
  }
  if (field.kind === "uuid" && field.ref) return <RefCell resource={field.ref} id={String(value)} />;
  if (field.kind === "uuid") return <span className="id">{shortId(value)}</span>;
  if (field.kind === "datetime") return <span className="cellmuted tnum">{fmtDate(value)}</span>;
  if (field.kind === "number") return <span className="tnum">{String(value ?? "")}</span>;
  return <span>{String(value ?? "")}</span>;
}

export function Resource({
  desc,
  onAdd,
  onEdit,
}: {
  desc: ResourceDesc;
  onAdd: () => void;
  onEdit: (row: Row) => void;
}) {
  const qc = useQueryClient();
  const columns = desc.fields.filter((f) => f.kind !== "password");
  const boolFields = desc.fields.filter((f) => f.kind === "boolean");

  const [q, setQ] = useState("");
  const [page, setPage] = useState(0);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [boolFilter, setBoolFilter] = useState<Record<string, "all" | "yes" | "no">>({});

  const list = useQuery<Row[]>({ queryKey: [desc.path], queryFn: () => api.list(desc.path) });
  const items = list.data ?? [];

  const remove = useMutation({
    mutationFn: (ids: string[]) => Promise.all(ids.map((id) => api.remove(desc.path, id))),
    onSuccess: (_r, ids) => {
      qc.invalidateQueries({ queryKey: [desc.path] });
      setSelected(new Set());
      toast(ids.length > 1 ? `Deleted ${ids.length} ${desc.path}` : "Deleted");
    },
    onError: (e) => toast(String(e)),
  });

  const filtered = useMemo(() => {
    return items.filter((it) => {
      for (const f of boolFields) {
        const want = boolFilter[f.name] ?? "all";
        if (want !== "all" && Boolean(it[f.name]) !== (want === "yes")) return false;
      }
      if (q && !JSON.stringify(it).toLowerCase().includes(q.toLowerCase())) return false;
      return true;
    });
  }, [items, q, boolFilter, boolFields]);

  const pages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const clampedPage = Math.min(page, pages - 1);
  const shown = filtered.slice(clampedPage * PAGE_SIZE, clampedPage * PAGE_SIZE + PAGE_SIZE);
  const allChecked = shown.length > 0 && shown.every((r) => selected.has(r.id));

  function toggleRow(id: string) {
    const next = new Set(selected);
    next.has(id) ? next.delete(id) : next.add(id);
    setSelected(next);
  }
  function toggleAll() {
    const next = new Set(selected);
    if (allChecked) shown.forEach((r) => next.delete(r.id));
    else shown.forEach((r) => next.add(r.id));
    setSelected(next);
  }

  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>{titleCase(desc.path)}</h1>
          <p>
            {filtered.length} {filtered.length === 1 ? "record" : "records"} in <code>{desc.path}</code>.
          </p>
        </div>
        <span className="spacer" />
        <button className="btn primary" onClick={onAdd}>
          <Icon name="plus" />
          Add {desc.name.toLowerCase()}
        </button>
      </div>

      <div className="board">
        <div className="card">
          <div className="toolbar">
            <div className="listsearch">
              <Icon name="search" />
              <input
                placeholder={`Filter ${desc.path}…`}
                value={q}
                onChange={(e) => {
                  setQ(e.target.value);
                  setPage(0);
                }}
              />
            </div>
            <span className="count">
              <b>{filtered.length}</b> of <b>{items.length}</b>
            </span>
          </div>

          {selected.size > 0 && (
            <div className="bulkbar">
              <span>{selected.size} selected</span>
              <span style={{ flex: 1 }} />
              <button className="btn sm" onClick={() => setSelected(new Set())}>
                Clear
              </button>
              <button className="btn sm danger" onClick={() => remove.mutate([...selected])}>
                <Icon name="trash" />
                Delete selected
              </button>
            </div>
          )}

          <div style={{ overflowX: "auto" }}>
            <table>
              <thead>
                <tr>
                  <th className="chk">
                    <input
                      type="checkbox"
                      checked={allChecked}
                      onChange={toggleAll}
                      aria-label="Select all"
                    />
                  </th>
                  {columns.map((c) => (
                    <th key={c.name}>{c.name}</th>
                  ))}
                  <th>Created</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {shown.map((item) => (
                  <tr key={item.id} className={selected.has(item.id) ? "sel" : ""}>
                    <td className="chk">
                      <input
                        type="checkbox"
                        checked={selected.has(item.id)}
                        onChange={() => toggleRow(item.id)}
                        aria-label="Select row"
                      />
                    </td>
                    {columns.map((c, i) => (
                      <td key={c.name}>
                        <div className={i === 0 ? "cell-title" : ""}>
                          <Cell field={c} value={item[c.name]} />
                        </div>
                        {i === 0 && <div className="id">{shortId(item.id)}</div>}
                      </td>
                    ))}
                    <td className="cellmuted tnum">{fmtDate(item.created_at)}</td>
                    <td className="actions">
                      <button className="btn sm ghost" onClick={() => onEdit(item)}>
                        <Icon name="edit" />
                        Edit
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>

            {shown.length === 0 && (
              <div className="empty">
                <Icon name="inbox" />
                <div>{list.isLoading ? "Loading…" : `No ${desc.path} yet.`}</div>
              </div>
            )}
          </div>

          <div className="tablefoot">
            <span>
              Rows per page: <b className="tnum">{PAGE_SIZE}</b>
            </span>
            <span className="spacer" />
            <span className="tnum">
              {filtered.length === 0 ? 0 : clampedPage * PAGE_SIZE + 1}–
              {Math.min((clampedPage + 1) * PAGE_SIZE, filtered.length)} of {filtered.length}
            </span>
            <div className="pager">
              <button
                disabled={clampedPage === 0}
                onClick={() => setPage(clampedPage - 1)}
                aria-label="Previous"
              >
                <Icon name="chevronLeft" />
              </button>
              <button aria-current="true">{clampedPage + 1}</button>
              <button
                disabled={clampedPage + 1 >= pages}
                onClick={() => setPage(clampedPage + 1)}
                aria-label="Next"
              >
                <Icon name="chevronRight" />
              </button>
            </div>
          </div>
        </div>

        <aside className="card filters">
          <h3>Filters</h3>
          {boolFields.length === 0 && (
            <div style={{ padding: "0 14px 14px", color: "var(--muted)", fontSize: 13 }}>
              No filters for this resource.
            </div>
          )}
          {boolFields.map((f) => {
            const val = boolFilter[f.name] ?? "all";
            const opts: Array<["all" | "yes" | "no", string]> = [
              ["all", "All"],
              ["yes", `${f.name}`],
              ["no", `Not ${f.name}`],
            ];
            return (
              <div className="fgroup" key={f.name}>
                <div className="cap">By {f.name}</div>
                {opts.map(([key, lbl]) => (
                  <button
                    key={key}
                    className={"opt" + (val === key ? " on" : "")}
                    onClick={() => {
                      setBoolFilter({ ...boolFilter, [f.name]: key });
                      setPage(0);
                    }}
                  >
                    <Icon name="check" className="tick" />
                    <span>{lbl}</span>
                  </button>
                ))}
              </div>
            );
          })}
          {(boolFields.length > 0 || q) && (
            <button
              className="btn sm ghost clear"
              onClick={() => {
                setBoolFilter({});
                setQ("");
                setPage(0);
              }}
            >
              Clear filters
            </button>
          )}
        </aside>
      </div>
    </div>
  );
}
