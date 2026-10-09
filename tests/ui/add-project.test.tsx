import { readdirSync } from "node:fs";
import { join } from "node:path";
import { screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { choose, dataDir, gitSubjects, renderApp, showArchived, writeTodo } from "./helpers";

async function openDialog(user: ReturnType<typeof renderApp>["user"]) {
  await user.click(await screen.findByRole("button", { name: "Add project" }));
  await screen.findByText("Add project", { selector: "h2" });
}

test("adding a project tracks it, names it after its folder, and commits it", async () => {
  const file = writeTodo("- [ ] one\n- [ ] two\n", "Garden");
  const { user } = renderApp({ dialogPath: file });
  await openDialog(user);
  expect(screen.getByRole("button", { name: "Add" })).toBeDisabled();

  await user.click(screen.getByRole("button", { name: "Choose a TODO.md file…" }));
  expect(screen.getByPlaceholderText("Project name")).toHaveValue("Garden");
  expect(screen.getByText(file)).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Add" }));

  expect(await screen.findByRole("button", { name: /^Garden/ })).toHaveTextContent("2 open");
  expect(screen.queryByText("Add project", { selector: "h2" })).not.toBeInTheDocument();
  expect(readdirSync(join(dataDir(), "projects"))).toHaveLength(1);
  expect(gitSubjects()[0]).toBe("add: Garden");
});

test("a learning project goes to the learning tab", async () => {
  const { user } = renderApp({ dialogPath: writeTodo("- [ ] read\n", "Physics") });
  await openDialog(user);
  await user.click(screen.getByRole("button", { name: "Choose a TODO.md file…" }));
  await choose(user, 0, "Learning");
  await user.click(screen.getByRole("button", { name: "Add" }));

  await waitFor(() =>
    expect(screen.queryByText("Add project", { selector: "h2" })).not.toBeInTheDocument(),
  );
  expect(screen.queryByRole("button", { name: /^Physics/ })).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Learning" }));
  expect(await screen.findByRole("button", { name: /^Physics/ })).toBeInTheDocument();
});

test("a project added as done is archived", async () => {
  const { user } = renderApp({ dialogPath: writeTodo("- [x] all\n", "Finished") });
  await openDialog(user);
  await user.click(screen.getByRole("button", { name: "Choose a TODO.md file…" }));
  await choose(user, 1, "Done");
  await user.click(screen.getByRole("button", { name: "Add" }));
  await showArchived(user);

  await user.click(await screen.findByRole("button", { name: "Archived (1)" }));
  expect(screen.getByRole("button", { name: /^Finished/ })).toHaveTextContent("· done");
});

test("cancel closes the dialog and resets it", async () => {
  const { user } = renderApp({ dialogPath: writeTodo("- [ ] x\n", "Draft") });
  await openDialog(user);
  await user.click(screen.getByRole("button", { name: "Choose a TODO.md file…" }));
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  await waitFor(() =>
    expect(screen.queryByText("Add project", { selector: "h2" })).not.toBeInTheDocument(),
  );

  await openDialog(user);
  expect(screen.getByRole("button", { name: "Choose a TODO.md file…" })).toBeInTheDocument();
  expect(screen.getByPlaceholderText("Project name")).toHaveValue("");
  expect(screen.queryByRole("button", { name: /^Draft/ })).not.toBeInTheDocument();
});
