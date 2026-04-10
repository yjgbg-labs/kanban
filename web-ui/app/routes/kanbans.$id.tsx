import { json, redirect, type ActionFunctionArgs, type LoaderFunctionArgs, type MetaFunction } from "@remix-run/node";
import { Form, Link, useLoaderData } from "@remix-run/react";

const API = process.env.SERVER_URL || "http://localhost:3001";

interface Kanban { id: string; title: string; description: string; created_at: string; }
interface Column { id: string; kanban_id: string; title: string; position: number; created_at: string; }
interface Card { id: string; column_id: string; title: string; description: string; position: number; created_at: string; }

export const meta: MetaFunction<typeof loader> = ({ data }) => [
  { title: data?.kanban.title || "Board" }
];

export async function loader({ params }: LoaderFunctionArgs) {
  const { id } = params;
  const [kanbanRes, columnsRes] = await Promise.all([
    fetch(`${API}/api/kanbans/${id}`),
    fetch(`${API}/api/kanbans/${id}/columns`),
  ]);
  if (!kanbanRes.ok) throw new Response("Not Found", { status: 404 });
  const kanban: Kanban = await kanbanRes.json();
  const columns: Column[] = await columnsRes.json();
  const cardsPerColumn = await Promise.all(
    columns.map((col) => fetch(`${API}/api/columns/${col.id}/cards`).then((r) => r.json() as Promise<Card[]>))
  );
  const columnsWithCards = columns.map((col, i) => ({ ...col, cards: cardsPerColumn[i] }));
  return json({ kanban, columns: columnsWithCards });
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { id: kanbanId } = params;
  const form = await request.formData();
  const intent = form.get("intent") as string;

  if (intent === "create-column") {
    const title = form.get("title") as string;
    if (!title?.trim()) return json({ error: "Title required" });
    await fetch(`${API}/api/kanbans/${kanbanId}/columns`, {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ title }),
    });
    return redirect(`/kanbans/${kanbanId}`);
  }

  if (intent === "delete-column") {
    const colId = form.get("column_id") as string;
    await fetch(`${API}/api/columns/${colId}`, { method: "DELETE" });
    return redirect(`/kanbans/${kanbanId}`);
  }

  if (intent === "create-card") {
    const columnId = form.get("column_id") as string;
    const title = form.get("title") as string;
    const description = form.get("description") as string;
    if (!title?.trim()) return json({ error: "Title required" });
    await fetch(`${API}/api/columns/${columnId}/cards`, {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ title, description }),
    });
    return redirect(`/kanbans/${kanbanId}`);
  }

  if (intent === "delete-card") {
    const cardId = form.get("card_id") as string;
    await fetch(`${API}/api/cards/${cardId}`, { method: "DELETE" });
    return redirect(`/kanbans/${kanbanId}`);
  }

  return json({ error: "Unknown action" });
}

export default function KanbanBoard() {
  const { kanban, columns } = useLoaderData<typeof loader>();

  return (
    <div style={{ minHeight: "100vh" }}>
      <div style={{ background: "white", padding: "1rem 2rem", boxShadow: "0 1px 3px rgba(0,0,0,.1)", display: "flex", alignItems: "center", gap: "1rem" }}>
        <Link to="/">← Boards</Link>
        <h1 style={{ fontSize: "1.4rem", margin: 0 }}>{kanban.title}</h1>
        {kanban.description && <span style={{ color: "#666", fontSize: ".9rem" }}>{kanban.description}</span>}
      </div>

      <div style={{ padding: "1.5rem", overflowX: "auto" }}>
        <div style={{ display: "flex", gap: "1rem", alignItems: "flex-start", minHeight: "60vh" }}>
          {columns.map((col) => (
            <div key={col.id} style={{ width: 280, minWidth: 280, background: "#ebecf0", borderRadius: 8, padding: "1rem" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: ".75rem" }}>
                <strong>{col.title}</strong>
                <Form method="post" style={{ display: "inline" }}>
                  <input type="hidden" name="intent" value="delete-column" />
                  <input type="hidden" name="column_id" value={col.id} />
                  <button type="submit" style={{ background: "none", border: "none", color: "#888", fontSize: "1rem", cursor: "pointer" }}
                    title="Delete column"
                    onClick={(e) => { if (!confirm("Delete column and all cards?")) e.preventDefault(); }}>
                    ✕
                  </button>
                </Form>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: ".5rem", marginBottom: "1rem" }}>
                {col.cards.map((card) => (
                  <div key={card.id} style={{ background: "white", borderRadius: 6, padding: ".75rem", boxShadow: "0 1px 2px rgba(0,0,0,.1)" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                      <strong style={{ fontSize: ".9rem" }}>{card.title}</strong>
                      <Form method="post" style={{ display: "inline", marginLeft: ".5rem" }}>
                        <input type="hidden" name="intent" value="delete-card" />
                        <input type="hidden" name="card_id" value={card.id} />
                        <button type="submit" style={{ background: "none", border: "none", color: "#aaa", cursor: "pointer", fontSize: ".8rem" }}
                          title="Delete card"
                          onClick={(e) => { if (!confirm("Delete this card?")) e.preventDefault(); }}>
                          ✕
                        </button>
                      </Form>
                    </div>
                    {card.description && <p style={{ fontSize: ".8rem", color: "#666", marginTop: ".25rem" }}>{card.description}</p>}
                  </div>
                ))}
              </div>

              <details>
                <summary style={{ cursor: "pointer", color: "#555", fontSize: ".85rem", listStyle: "none" }}>+ Add card</summary>
                <Form method="post" style={{ marginTop: ".5rem", display: "flex", flexDirection: "column", gap: ".4rem" }}>
                  <input type="hidden" name="intent" value="create-card" />
                  <input type="hidden" name="column_id" value={col.id} />
                  <input name="title" placeholder="Card title" required style={{ padding: ".4rem" }} />
                  <textarea name="description" placeholder="Description (optional)" style={{ minHeight: 60, padding: ".4rem", resize: "vertical" }} />
                  <button type="submit" className="btn btn-primary" style={{ padding: ".4rem .8rem", background: "#0066cc", color: "white", border: "none", borderRadius: 4 }}>Add</button>
                </Form>
              </details>
            </div>
          ))}

          <div style={{ width: 280, minWidth: 280 }}>
            <details style={{ background: "#ebecf0", borderRadius: 8, padding: "1rem" }}>
              <summary style={{ cursor: "pointer", fontWeight: "bold", listStyle: "none" }}>+ Add column</summary>
              <Form method="post" style={{ marginTop: ".75rem", display: "flex", flexDirection: "column", gap: ".5rem" }}>
                <input type="hidden" name="intent" value="create-column" />
                <input name="title" placeholder="Column title" required />
                <button type="submit" className="btn btn-primary" style={{ padding: ".4rem .8rem", background: "#0066cc", color: "white", border: "none", borderRadius: 4 }}>Add Column</button>
              </Form>
            </details>
          </div>
        </div>
      </div>
    </div>
  );
}
