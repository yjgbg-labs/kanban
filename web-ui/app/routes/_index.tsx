import { json, redirect, type ActionFunctionArgs, type MetaFunction } from "@remix-run/node";
import { Form, Link, useLoaderData, useActionData } from "@remix-run/react";

const API = process.env.SERVER_URL || "http://localhost:3001";

interface Kanban {
  id: string;
  title: string;
  description: string;
  created_at: string;
}

export const meta: MetaFunction = () => [{ title: "Kanban App" }];

export async function loader() {
  const res = await fetch(`${API}/api/kanbans`);
  const kanbans: Kanban[] = await res.json();
  return json({ kanbans });
}

export async function action({ request }: ActionFunctionArgs) {
  const form = await request.formData();
  const intent = form.get("intent");

  if (intent === "create") {
    const title = form.get("title") as string;
    const description = form.get("description") as string;
    if (!title?.trim()) return json({ error: "Title is required" });
    const res = await fetch(`${API}/api/kanbans`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ title, description }),
    });
    const kanban: Kanban = await res.json();
    return redirect(`/kanbans/${kanban.id}`);
  }

  if (intent === "delete") {
    const id = form.get("id") as string;
    await fetch(`${API}/api/kanbans/${id}`, { method: "DELETE" });
    return redirect("/");
  }

  return json({ error: "Unknown action" });
}

export default function Index() {
  const { kanbans } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();

  return (
    <div className="container">
      <h1>&#x1F4CB; Kanban Boards</h1>

      <div className="card" style={{ marginBottom: "2rem" }}>
        <h2>Create New Board</h2>
        <Form method="post" style={{ display: "flex", flexDirection: "column", gap: ".75rem" }}>
          <input type="hidden" name="intent" value="create" />
          {actionData && "error" in actionData && <p className="error">{actionData.error}</p>}
          <div>
            <label>Title</label>
            <input name="title" placeholder="My Project Board" required />
          </div>
          <div>
            <label>Description</label>
            <textarea name="description" placeholder="Optional description..." />
          </div>
          <div>
            <button type="submit" className="btn btn-primary">Create Board</button>
          </div>
        </Form>
      </div>

      {kanbans.length === 0 ? (
        <p style={{ color: "#666" }}>No boards yet. Create one above!</p>
      ) : (
        <div style={{ display: "grid", gap: "1rem", gridTemplateColumns: "repeat(auto-fill, minmax(300px,1fr))" }}>
          {kanbans.map((k) => (
            <div key={k.id} className="card" style={{ display: "flex", flexDirection: "column", gap: ".5rem" }}>
              <h2><Link to={`/kanbans/${k.id}`}>{k.title}</Link></h2>
              {k.description && <p style={{ color: "#555" }}>{k.description}</p>}
              <div style={{ marginTop: "auto", display: "flex", gap: ".5rem", paddingTop: ".5rem" }}>
                <Link to={`/kanbans/${k.id}`} className="btn btn-primary btn">Open</Link>
                <Form method="post" style={{ display: "inline" }}>
                  <input type="hidden" name="intent" value="delete" />
                  <input type="hidden" name="id" value={k.id} />
                  <button type="submit" className="btn btn-danger"
                    onClick={(e) => { if (!confirm("Delete this board?")) e.preventDefault(); }}>
                    Delete
                  </button>
                </Form>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
