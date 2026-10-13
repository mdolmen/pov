import { appendFileSync } from "node:fs";
import { fireEvent, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { addProject, inboxTasks, openProject, renderApp, taskPane, triageButton } from "./helpers";

test("a task added outside the app shows, with its inbox mark, when the window gains focus", async () => {
  const project = await addProject({ name: "P", content: "- [ ] alpha\n" });
  const { user } = renderApp();
  await openProject(user, "P");

  // As `pov mcp` does: write the file, then mark the task.
  appendFileSync(project.file_path, "- [ ] added\n");
  await inboxTasks(project.id, "added");
  expect(taskPane().queryByText("added")).not.toBeInTheDocument();

  fireEvent.focus(window);

  await waitFor(() => expect(taskPane().getByText("added")).toBeInTheDocument());
  expect(triageButton("added", "Keep")).toBeInTheDocument();
});
