#!/usr/bin/env node
const { Command } = require("commander");
const axios = require("axios");

const program = new Command();
const BASE_URL = process.env.SERVER_URL || "http://localhost:3001";
const api = axios.create({ baseURL: BASE_URL });

function print(data) {
  console.log(JSON.stringify(data, null, 2));
}

async function run(fn) {
  try {
    await fn();
  } catch (e) {
    const msg = e.response?.data || e.message;
    console.error("Error:", typeof msg === "object" ? JSON.stringify(msg) : msg);
    process.exit(1);
  }
}

program
  .name("kanban")
  .description("Kanban CLI - manage kanbans, columns, and cards")
  .version("1.0.0");

const kanban = program.command("kanban").description("Manage kanbans");

kanban.command("list")
  .description("List all kanbans")
  .action(() => run(async () => {
    const { data } = await api.get("/api/kanbans");
    print(data);
  }));

kanban.command("get <id>")
  .description("Get a kanban by ID")
  .action((id) => run(async () => {
    const { data } = await api.get(`/api/kanbans/${id}`);
    print(data);
  }));

kanban.command("create")
  .description("Create a new kanban")
  .requiredOption("-t, --title <title>", "Kanban title")
  .option("-d, --description <description>", "Kanban description", "")
  .action((opts) => run(async () => {
    const { data } = await api.post("/api/kanbans", { title: opts.title, description: opts.description });
    print(data);
  }));

kanban.command("update <id>")
  .description("Update a kanban")
  .option("-t, --title <title>", "New title")
  .option("-d, --description <description>", "New description")
  .action((id, opts) => run(async () => {
    const body = {};
    if (opts.title) body.title = opts.title;
    if (opts.description) body.description = opts.description;
    const { data } = await api.put(`/api/kanbans/${id}`, body);
    print(data);
  }));

kanban.command("delete <id>")
  .description("Delete a kanban")
  .action((id) => run(async () => {
    await api.delete(`/api/kanbans/${id}`);
    console.log("Deleted.");
  }));

const column = program.command("column").description("Manage columns");

column.command("list <kanban-id>")
  .description("List columns in a kanban")
  .action((kanbanId) => run(async () => {
    const { data } = await api.get(`/api/kanbans/${kanbanId}/columns`);
    print(data);
  }));

column.command("get <id>")
  .description("Get a column by ID")
  .action((id) => run(async () => {
    const { data } = await api.get(`/api/columns/${id}`);
    print(data);
  }));

column.command("create <kanban-id>")
  .description("Create a column in a kanban")
  .requiredOption("-t, --title <title>", "Column title")
  .action((kanbanId, opts) => run(async () => {
    const { data } = await api.post(`/api/kanbans/${kanbanId}/columns`, { title: opts.title });
    print(data);
  }));

column.command("update <id>")
  .description("Update a column")
  .option("-t, --title <title>", "New title")
  .action((id, opts) => run(async () => {
    const body = {};
    if (opts.title) body.title = opts.title;
    const { data } = await api.put(`/api/columns/${id}`, body);
    print(data);
  }));

column.command("delete <id>")
  .description("Delete a column")
  .action((id) => run(async () => {
    await api.delete(`/api/columns/${id}`);
    console.log("Deleted.");
  }));

const card = program.command("card").description("Manage cards");

card.command("list <column-id>")
  .description("List cards in a column")
  .action((columnId) => run(async () => {
    const { data } = await api.get(`/api/columns/${columnId}/cards`);
    print(data);
  }));

card.command("get <id>")
  .description("Get a card by ID")
  .action((id) => run(async () => {
    const { data } = await api.get(`/api/cards/${id}`);
    print(data);
  }));

card.command("create <column-id>")
  .description("Create a card in a column")
  .requiredOption("-t, --title <title>", "Card title")
  .option("-d, --description <description>", "Card description", "")
  .action((columnId, opts) => run(async () => {
    const { data } = await api.post(`/api/columns/${columnId}/cards`, { title: opts.title, description: opts.description });
    print(data);
  }));

card.command("update <id>")
  .description("Update a card")
  .option("-t, --title <title>", "New title")
  .option("-d, --description <description>", "New description")
  .action((id, opts) => run(async () => {
    const body = {};
    if (opts.title) body.title = opts.title;
    if (opts.description) body.description = opts.description;
    const { data } = await api.put(`/api/cards/${id}`, body);
    print(data);
  }));

card.command("delete <id>")
  .description("Delete a card")
  .action((id) => run(async () => {
    await api.delete(`/api/cards/${id}`);
    console.log("Deleted.");
  }));

program.parse(process.argv);
