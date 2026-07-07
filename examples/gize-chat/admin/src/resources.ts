import { z } from "zod";

export type FieldKind =
  | "string"
  | "number"
  | "boolean"
  | "uuid"
  | "datetime"
  | "email"
  | "password";

export interface FieldDesc {
  name: string;
  kind: FieldKind;
  /** For a foreign key: the target resource path (e.g. "users"). */
  ref?: string;
}

export interface ResourceDesc {
  name: string;
  path: string;
  fields: FieldDesc[];
  createSchema: z.ZodTypeAny;
}

export const resources: ResourceDesc[] = [
  {
    name: "Message",
    path: "messages",
    fields: [
      { name: "content", kind: "string" },
      { name: "username", kind: "string" },
    ],
    createSchema: z.object({
      content: z.string().min(1),
      username: z.string().min(1),
    }),
  },
  {
    name: "User",
    path: "users",
    fields: [
      { name: "name", kind: "string" },
      { name: "email", kind: "email" },
      { name: "password", kind: "password" },
      { name: "is_admin", kind: "boolean" },
    ],
    createSchema: z.object({
      name: z.string().min(1),
      email: z.string().email(),
      password: z.string().min(8),
      is_admin: z.boolean(),
    }),
  },
];
