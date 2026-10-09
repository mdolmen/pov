import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { addProject, inOrder, openProject, projectsPane, renderApp, taskPane, writeTodo } from "./helpers";

test("a project shows its headings and tasks in file order, done ones in place", async () => {
  await addProject({
    name: "Book",
    content: [
      "# Book",
      "## Draft",
      "- [ ] outline",
      "- [ ] chapter one",
      "## Published",
      "- [x] cover",
      "## Review",
      "### Deep",
      "- [ ] proofread",
      "",
    ].join("\n"),
  });
  const { user } = renderApp();
  await openProject(user, "Book");

  expect(await taskPane().findByText("outline")).toBeInTheDocument();
  expect(
    inOrder("Draft", "outline", "chapter one", "Published", "cover", "Review", "Deep", "proofread"),
  ).toBe(true);
  // Milestones show their progress; the level-1 heading is not shown, only the pane title.
  expect(taskPane().getByText("0 of 2 done")).toBeInTheDocument();
  expect(taskPane().getByText("1 of 1 done")).toBeInTheDocument();
  expect(taskPane().getByText("0 of 1 done")).toBeInTheDocument();
  expect(taskPane().getAllByText("Book")).toHaveLength(1);
});

test("closing the pane keeps the tab", async () => {
  await addProject({ name: "Work" });
  await addProject({ name: "Maths", type: "learning" });
  const { user } = renderApp();
  await user.click(await screen.findByRole("button", { name: "learning" }));
  await openProject(user, "Maths");

  await user.click(screen.getByRole("button", { name: "Close project" }));

  expect(screen.getByText("Choose a project to show its tasks")).toBeInTheDocument();
  expect(projectsPane().getByRole("button", { name: /^Maths/ })).toBeInTheDocument();
  expect(projectsPane().queryByRole("button", { name: /^Work/ })).not.toBeInTheDocument();
});

test("Esc closes the pane", async () => {
  await addProject({ name: "Work" });
  const { user } = renderApp();
  await openProject(user, "Work");

  await user.keyboard("{Escape}");

  expect(screen.getByText("Choose a project to show its tasks")).toBeInTheDocument();
});

test("task text renders inline markdown, and links open in the browser", async () => {
  await addProject({
    name: "Docs",
    content: "- [ ] read **the spec** and `cargo test` at [the docs](https://example.com/docs)\n",
  });
  const { user, calls } = renderApp();
  await openProject(user, "Docs");

  expect((await screen.findByText("the spec")).tagName).toBe("STRONG");
  expect(screen.getByText("cargo test").tagName).toBe("CODE");
  await user.click(screen.getByRole("link", { name: "the docs" }));

  expect(calls).toContainEqual(["plugin:opener|open_url", { url: "https://example.com/docs" }]);
});

test("the edit button opens the file in the editor", async () => {
  const project = await addProject({ name: "Edit" });
  const { user, calls } = renderApp();
  await openProject(user, "Edit");

  await user.click(screen.getByRole("button", { name: "Open in default text editor" }));

  expect(calls).toContainEqual(["open_in_editor", { path: project.file_path }]);
});

test("a file that isn't text has no tasks", async () => {
  const pdf = join(writeTodo("").replace(/TODO\.md$/, ""), "book.pdf");
  writeFileSync(pdf, Buffer.from("%PDF-1.4\n\xe2\x28\xa1 binary\n", "latin1"));
  await addProject({ name: "Scan", file: pdf, type: "learning" });
  const { user } = renderApp();
  await user.click(await screen.findByRole("button", { name: "learning" }));
  await openProject(user, "Scan");

  expect(await screen.findByText("No tasks.")).toBeInTheDocument();
});
