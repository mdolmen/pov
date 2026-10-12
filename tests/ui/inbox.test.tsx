import { readFileSync } from "node:fs";
import { screen, waitFor, within } from "@testing-library/react";
import { expect, test } from "vitest";
import {
  addProject,
  checkbox,
  inboxTasks,
  nextButton,
  openProject,
  projectTiles,
  renderApp,
  struck,
  taskPane,
  tileName,
  triageButton,
} from "./helpers";

/** The band on a task's row: "next", "inbox" or null. */
const band = (text: string) => {
  const row = taskPane().getByText(text).closest("[data-task]") as HTMLElement;
  return row.style.boxShadow.match(/--pov-(next|inbox)/)?.[1] ?? null;
};

const TODO = "- [ ] alpha\n- [ ] beta\n- [ ] gamma\n";

/** Opens project "P", tracking `content` with `inbox` tasks in its inbox. */
async function setup(content: string, ...inbox: string[]) {
  const p = await addProject({ name: "P", content });
  await inboxTasks(p.id, ...inbox);
  const { user } = renderApp();
  await openProject(user, "P");
  return Object.assign(user, { file: p.file_path });
}

test("inbox tasks carry the amber band; backlog tasks can't be put in the inbox", async () => {
  await setup(TODO, "beta");

  expect(band("beta")).toBe("inbox");
  expect(band("alpha")).toBe(null);
  expect(triageButton("beta", "Keep")).toBeInTheDocument();
  expect(within(taskPane().getByText("alpha").closest("[data-task]") as HTMLElement)
    .queryByRole("button", { name: "Keep" })).not.toBeInTheDocument();
});

test("Keep moves an inbox task to the backlog", async () => {
  const user = await setup(TODO, "beta");

  await user.click(triageButton("beta", "Keep"));

  expect(band("beta")).toBe(null);
  await user.click(screen.getByRole("button", { name: "Close project" }));
  await openProject(user, "P");
  expect(band("beta")).toBe(null);
});

test("Mark next takes a task out of the inbox, and it stays next", async () => {
  const user = await setup(TODO, "beta");

  await user.click(nextButton("beta"));
  expect(band("beta")).toBe("next");
  expect(taskPane().queryByRole("button", { name: "Keep" })).not.toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Close project" }));
  await openProject(user, "P");
  expect(band("beta")).toBe("next");
});

test("double-clicking an inbox task marks it next", async () => {
  const user = await setup(TODO, "gamma");

  await user.dblClick(taskPane().getByText("gamma"));

  expect(band("gamma")).toBe("next");
});

test("the tile shows a badge with the inbox count", async () => {
  const user = await setup(TODO, "alpha", "beta");
  const tile = () => projectTiles().getByRole("button", { name: tileName("P") });
  expect(within(tile()).getByLabelText("2 in inbox")).toHaveTextContent("2");

  await user.click(nextButton("beta"));
  expect(await projectTiles().findByRole("button", { name: /^P1 next of 3 open/ })).toBeInTheDocument();
  expect(await within(tile()).findByLabelText("1 in inbox")).toBeInTheDocument();

  await user.click(triageButton("alpha", "Keep"));
  await waitFor(() => expect(within(tile()).queryByLabelText(/in inbox$/)).not.toBeInTheDocument());
});

test("inbox tasks stay out of Next up", async () => {
  await setup("## M\n- [ ] alpha\n", "alpha");

  expect(await screen.findByText("Nothing marked next")).toBeInTheDocument();
});

test("checking an inbox task strikes it and takes it out of the inbox", async () => {
  const user = await setup(TODO, "beta");

  await user.click(checkbox("beta"));

  await waitFor(() => expect(struck("beta")).toBe(true));
  expect(band("beta")).toBe(null);
  await waitFor(() =>
    expect(within(projectTiles().getByRole("button", { name: tileName("P") }))
      .queryByLabelText(/in inbox$/)).not.toBeInTheDocument(),
  );
});

test("Inbox shows only the inbox tasks, under their milestones", async () => {
  const user = await setup("## One\n- [ ] alpha\n- [x] beta\n## Two\n- [ ] gamma\n- [ ] delta\n", "delta");
  await user.click(nextButton("alpha"));

  const filter = screen.getByRole("button", { name: "Inbox", pressed: false });
  await user.click(filter);

  expect(filter).toHaveAttribute("aria-pressed", "true");
  expect(taskPane().getByText("delta")).toBeInTheDocument();
  for (const text of ["alpha", "beta", "gamma"]) {
    expect(taskPane().queryByText(text)).not.toBeInTheDocument();
  }
  expect(taskPane().getByText("Two")).toBeInTheDocument();
  expect(taskPane().queryByText("One")).not.toBeInTheDocument();

  await user.click(filter);
  expect(taskPane().getByText("alpha")).toBeInTheDocument();
  expect(taskPane().getByText("beta")).toBeInTheDocument();
});

test("no Inbox filter without inbox tasks", async () => {
  await setup(TODO);
  expect(screen.queryByRole("button", { name: "Inbox" })).not.toBeInTheDocument();
});

test("triaging the last inbox task turns the filter off", async () => {
  const user = await setup(TODO, "beta");
  await user.click(screen.getByRole("button", { name: "Inbox", pressed: false }));

  await user.click(triageButton("beta", "Keep"));

  expect(screen.queryByRole("button", { name: "Inbox" })).not.toBeInTheDocument();
  expect(taskPane().getByText("alpha")).toBeInTheDocument();
});

test("× deletes an inbox task from the list and the file", async () => {
  const user = await setup("- [ ] alpha\n- [ ] beta\n  - [ ] beta one\n- [ ] gamma\n", "beta");
  expect(within(taskPane().getByText("alpha").closest("[data-task]") as HTMLElement)
    .queryByRole("button", { name: "Delete task" })).not.toBeInTheDocument();

  await user.click(triageButton("beta", "Delete task"));

  expect(taskPane().queryByText("beta")).not.toBeInTheDocument();
  expect(taskPane().queryByText("beta one")).not.toBeInTheDocument();
  expect(await screen.findByRole("status")).toHaveTextContent("Deleted “beta”");
  await waitFor(() => expect(readFileSync(user.file, "utf8")).toBe("- [ ] alpha\n- [ ] gamma\n"));
  await waitFor(() =>
    expect(within(projectTiles().getByRole("button", { name: tileName("P") }))
      .queryByLabelText(/in inbox$/)).not.toBeInTheDocument(),
  );
});

test("Undo puts the task back in the file, in place, and in the inbox", async () => {
  const content = "- [ ] alpha\n- [ ] beta\n  - [ ] beta one\n- [ ] gamma\n";
  const user = await setup(content, "beta");
  await user.click(triageButton("beta", "Delete task"));
  await user.click(await screen.findByRole("button", { name: "Undo" }));

  expect(await taskPane().findByText("beta")).toBeInTheDocument();
  expect(readFileSync(user.file, "utf8")).toBe(content);
  expect(band("beta")).toBe("inbox");
  expect(screen.queryByRole("status")).not.toBeInTheDocument();
  expect(await within(projectTiles().getByRole("button", { name: tileName("P") }))
    .findByLabelText("1 in inbox")).toBeInTheDocument();
});

test("a new delete replaces the undo offer", async () => {
  const user = await setup(TODO, "alpha", "beta");

  await user.click(triageButton("alpha", "Delete task"));
  await screen.findByRole("status");
  await user.click(triageButton("beta", "Delete task"));
  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Deleted “beta”"));
  await user.click(screen.getByRole("button", { name: "Undo" }));

  expect(await taskPane().findByText("beta")).toBeInTheDocument();
  expect(taskPane().queryByText("alpha")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Undo" })).not.toBeInTheDocument();
});
