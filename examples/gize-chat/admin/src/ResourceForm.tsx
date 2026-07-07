import { useForm, Controller } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, labelOf } from "./api";
import { Icon } from "./icons";
import { toast } from "./toast";
import type { FieldDesc, ResourceDesc } from "./resources";

type Row = Record<string, any>;

function defaultFor(field: FieldDesc, row: Row | null): any {
  if (row && row[field.name] !== undefined && row[field.name] !== null) return row[field.name];
  if (field.kind === "boolean") return false;
  if (field.kind === "number") return 0;
  return "";
}

function FkSelect({
  resource,
  value,
  onChange,
}: {
  resource: string;
  value: string;
  onChange: (v: string) => void;
}) {
  const { data } = useQuery<Row[]>({ queryKey: [resource], queryFn: () => api.list(resource) });
  return (
    <div className="selectwrap">
      <select value={value ?? ""} onChange={(e) => onChange(e.target.value)}>
        <option value="">Select…</option>
        {(data ?? []).map((r) => (
          <option key={r.id} value={r.id}>
            {labelOf(r)}
          </option>
        ))}
      </select>
      <Icon name="chevronDown" />
    </div>
  );
}

export function ResourceForm({
  desc,
  row,
  onClose,
}: {
  desc: ResourceDesc;
  row: Row | null;
  onClose: () => void;
}) {
  const qc = useQueryClient();
  const editing = row !== null;

  const defaults: Row = {};
  for (const f of desc.fields) defaults[f.name] = defaultFor(f, row);

  const { register, handleSubmit, control, reset, formState } = useForm<Row>({
    resolver: zodResolver(desc.createSchema) as any,
    defaultValues: defaults,
  });
  const errors = formState.errors;

  const save = useMutation({
    mutationFn: (values: Row) =>
      editing ? api.update(desc.path, row!.id, values) : api.create(desc.path, values),
    onSuccess: () => qc.invalidateQueries({ queryKey: [desc.path] }),
    onError: (e) => toast(String(e)),
  });

  const remove = useMutation({
    mutationFn: () => api.remove(desc.path, row!.id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: [desc.path] });
      toast("Deleted");
      onClose();
    },
    onError: (e) => toast(String(e)),
  });

  function onSubmit(mode: "close" | "another") {
    return handleSubmit(async (values) => {
      await save.mutateAsync(values);
      toast(editing ? "Saved" : "Created");
      if (mode === "another") {
        reset(defaults);
      } else {
        onClose();
      }
    });
  }

  const noun = desc.name.toLowerCase();

  return (
    <aside className="drawer show" role="dialog" aria-modal="true" aria-label={`${editing ? "Edit" : "Add"} ${noun}`}>
      <div className="drawer__head">
        <div>
          <h2>
            {editing ? "Edit" : "Add"} {noun}
          </h2>
          {editing && <div className="sub mono">id: {String(row!.id).slice(0, 8)}…</div>}
        </div>
        <span className="spacer" />
        <button className="iconbtn" onClick={onClose} aria-label="Close">
          <Icon name="x" />
        </button>
      </div>

      <form
        className="drawer__body"
        id="resource-form"
        onSubmit={(e) => {
          e.preventDefault();
          onSubmit("close")();
        }}
      >
        <fieldset className="fieldset">
          <legend>Fields</legend>
          {desc.fields.map((f) => {
            const err = errors[f.name];
            return (
              <div className={"frow" + (err ? " err" : "")} key={f.name}>
                {f.kind === "boolean" ? (
                  <div className="togglerow">
                    <Controller
                      control={control}
                      name={f.name}
                      render={({ field }) => (
                        <button
                          type="button"
                          className="switch"
                          role="switch"
                          aria-checked={Boolean(field.value)}
                          onClick={() => field.onChange(!field.value)}
                        />
                      )}
                    />
                    <label style={{ margin: 0 }}>{f.name}</label>
                  </div>
                ) : (
                  <>
                    <label>
                      {f.name}
                      {f.kind !== "uuid" && <span className="req"> *</span>}
                    </label>
                    {f.kind === "uuid" && f.ref ? (
                      <Controller
                        control={control}
                        name={f.name}
                        render={({ field }) => (
                          <FkSelect resource={f.ref!} value={field.value} onChange={field.onChange} />
                        )}
                      />
                    ) : f.kind === "password" ? (
                      <input type="password" autoComplete="new-password" {...register(f.name)} />
                    ) : f.kind === "number" ? (
                      <input type="number" step="any" {...register(f.name)} />
                    ) : f.kind === "email" ? (
                      <input type="email" {...register(f.name)} />
                    ) : f.name === "body" || f.name === "description" || f.name === "content" ? (
                      <textarea {...register(f.name)} />
                    ) : (
                      <input type="text" {...register(f.name)} />
                    )}
                    {f.kind === "uuid" && f.ref && (
                      <div className="field-help">belongs_to → {f.ref}. Loaded from the related table.</div>
                    )}
                    {err && <div className="msg">{String(err.message ?? "Invalid value")}</div>}
                  </>
                )}
              </div>
            );
          })}
        </fieldset>
      </form>

      <div className="drawer__foot">
        {editing && (
          <button
            className="btn danger"
            onClick={() => {
              if (confirm(`Delete this ${noun}? This cannot be undone.`)) remove.mutate();
            }}
          >
            <Icon name="trash" />
            Delete
          </button>
        )}
        <span className="spacer" />
        {!editing && (
          <button className="btn" disabled={save.isPending} onClick={() => onSubmit("another")()}>
            Save &amp; add another
          </button>
        )}
        <button className="btn primary" disabled={save.isPending} onClick={() => onSubmit("close")()}>
          {save.isPending ? "Saving…" : "Save"}
        </button>
      </div>
    </aside>
  );
}
