import { screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { addProject, renderApp } from "./helpers";

const backdrop = () => document.querySelector('[style*="rgba(0, 0, 0, 0.08)"]') as HTMLElement;

test("the menu opens the sidebar, clicking outside closes it", async () => {
  const { user } = renderApp();
  await user.click(await screen.findByRole("button", { name: "Menu" }));
  expect(screen.getByText("Settings")).toBeInTheDocument();

  await user.click(backdrop());
  expect(screen.queryByText("Settings")).not.toBeInTheDocument();
});

test("filters show or hide open and archived projects, per tab", async () => {
  await addProject({ name: "Live" });
  await addProject({ name: "Old", status: "archived", sub_status: "paused" });
  await addProject({ name: "Course", type: "learning" });
  const { user } = renderApp();
  await screen.findByRole("button", { name: /^Live/ });

  await user.click(screen.getByRole("button", { name: "Menu" }));
  const [projectsOpen] = screen.getAllByRole("button", { name: "Open" });
  const [projectsArchived] = screen.getAllByRole("button", { name: "Archived" });
  await user.click(projectsOpen);
  await user.click(projectsArchived);
  await user.click(backdrop());

  expect(screen.queryByRole("button", { name: /^Live/ })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Archived (1)" })).toBeInTheDocument();

  // Learning keeps its own filters.
  await user.click(screen.getByRole("button", { name: "learning" }));
  expect(await screen.findByRole("button", { name: /^Course/ })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /^Archived/ })).not.toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(screen.getAllByRole("button", { name: "Open" })[1]);
  await user.click(backdrop());
  expect(screen.queryByRole("button", { name: /^Course/ })).not.toBeInTheDocument();
});

test("settings opens the settings page", async () => {
  const { user } = renderApp();
  await user.click(await screen.findByRole("button", { name: "Menu" }));
  await user.click(screen.getByRole("button", { name: "Settings" }));

  expect(await screen.findByText("Install CLI", { selector: "p" })).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Back" }));
  expect(await screen.findByRole("button", { name: "Menu" })).toBeInTheDocument();
});
