import { existsSync } from "node:fs";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { addProject, choose, getProject, openProject, renderApp } from "./helpers";

async function openSettings(user: ReturnType<typeof renderApp>["user"], name: string) {
  await openProject(user, name);
  await user.click(screen.getByRole("button", { name: "Project settings" }));
  await screen.findByText("Project settings", { selector: "h2" });
}

const closed = () =>
  waitFor(() =>
    expect(screen.queryByText("Project settings", { selector: "h2" })).not.toBeInTheDocument(),
  );

test("renaming updates the header and the list; save needs a change", async () => {
  await addProject({ name: "Old name" });
  const { user } = renderApp();
  await openSettings(user, "Old name");
  expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();

  const name = screen.getByLabelText("Name");
  await user.clear(name);
  await user.type(name, "New name");
  await user.click(screen.getByRole("button", { name: "Save" }));
  await closed();

  expect(screen.getByText("New name")).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Close project" }));
  expect(await screen.findByRole("button", { name: /^New name/ })).toBeInTheDocument();
});

test("pausing archives the project with an optional date", async () => {
  const project = await addProject({ name: "Side" });
  const { user } = renderApp();
  await openSettings(user, "Side");
  expect(screen.queryByText(/Paused until/)).not.toBeInTheDocument();

  await choose(user, 0, "Paused");
  fireEvent.change(screen.getByLabelText(/Paused until/), { target: { value: "2026-12-01" } });
  await user.click(screen.getByRole("button", { name: "Save" }));
  await closed();

  expect(await getProject(project.id)).toMatchObject({
    status: "archived",
    sub_status: "paused",
    paused_until: "2026-12-01",
  });
});

test("done and canceled archive the project", async () => {
  const done = await addProject({ name: "Shipped" });
  const dropped = await addProject({ name: "Dropped" });
  const { user } = renderApp();

  await openSettings(user, "Shipped");
  await choose(user, 0, "Done");
  await user.click(screen.getByRole("button", { name: "Save" }));
  await closed();
  await user.click(screen.getByRole("button", { name: "Close project" }));

  await openSettings(user, "Dropped");
  await choose(user, 0, "Canceled");
  await user.click(screen.getByRole("button", { name: "Save" }));
  await closed();

  expect(await getProject(done.id)).toMatchObject({ status: "archived", sub_status: "done" });
  expect(await getProject(dropped.id)).toMatchObject({
    status: "archived",
    sub_status: "canceled",
  });
});

test("delete asks for confirmation and keeps the original file", async () => {
  const project = await addProject({ name: "Gone" });
  const { user } = renderApp();
  await openSettings(user, "Gone");

  await user.click(screen.getByRole("button", { name: "Delete project" }));
  expect(screen.getByText("Delete this project?")).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.getByRole("button", { name: "Delete project" })).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Delete project" }));
  await user.click(screen.getByRole("button", { name: "Delete" }));

  expect(await screen.findByText("No projects yet.")).toBeInTheDocument();
  expect(existsSync(project.file_path)).toBe(true);
});
