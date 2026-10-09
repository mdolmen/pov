import { utimesSync } from "node:fs";
import { screen, waitFor, within } from "@testing-library/react";
import { expect, test } from "vitest";
import {
  addProject,
  daysAgo,
  getProject,
  openProject,
  taskPane,
  recordTime,
  renderApp,
  today,
} from "./helpers";

type User = ReturnType<typeof renderApp>["user"];

async function openLearning(user: User, name: string) {
  await user.click(await screen.findByRole("button", { name: "Learning" }));
  await openProject(user, name);
}

async function openRecordTime(user: User) {
  await user.click(screen.getByRole("button", { name: "Record time" }));
  const title = await screen.findByText("Record time", { selector: "h2" });
  const dialog = within(title.closest('[role="dialog"]') as HTMLElement);
  // The dialog focuses its first field asynchronously; typing before that lands elsewhere.
  await waitFor(() => expect(dialog.getByDisplayValue(today())).toHaveFocus());
  return dialog;
}

const closed = () =>
  waitFor(() =>
    expect(screen.queryByText("Record time", { selector: "h2" })).not.toBeInTheDocument(),
  );

test("only learning projects track time", async () => {
  await addProject({ name: "Work" });
  await addProject({ name: "Maths", type: "learning" });
  const { user } = renderApp();

  await openProject(user, "Work");
  expect(screen.queryByRole("button", { name: "Record time" })).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Close project" }));

  await openLearning(user, "Maths");
  expect(screen.getByRole("button", { name: "Record time" })).toBeInTheDocument();
});

test("the duration steps by 15 minutes, never below 15", async () => {
  await addProject({ name: "Maths", type: "learning" });
  const { user } = renderApp();
  await openLearning(user, "Maths");
  const dialog = await openRecordTime(user);

  expect(dialog.getByDisplayValue(today())).toBeInTheDocument();
  expect(dialog.getByText("1h")).toBeInTheDocument();
  for (let i = 0; i < 3; i++) await user.click(dialog.getByRole("button", { name: "Less time" }));
  expect(dialog.getByText("15min")).toBeInTheDocument();
  expect(dialog.getByRole("button", { name: "Less time" })).toBeDisabled();
  for (let i = 0; i < 4; i++) await user.click(dialog.getByRole("button", { name: "More time" }));
  expect(dialog.getByText("1h15")).toBeInTheDocument();
});

test("recorded time shows in the heatmap and counts as activity", async () => {
  const project = await addProject({ name: "Maths", type: "learning" });
  const old = new Date(Date.now() - 90 * 86_400_000);
  utimesSync(project.file_path, old, old);
  expect((await getProject(project.id)).activity).toBe("older");
  const { user } = renderApp();
  await openLearning(user, "Maths");
  expect(await screen.findByText(`No time on ${today()}`)).toBeInTheDocument();

  const dialog = await openRecordTime(user);
  await user.click(dialog.getByRole("button", { name: "More time" }));
  await user.type(dialog.getByPlaceholderText("Topic (optional)"), "Proba");
  await user.click(dialog.getByRole("button", { name: "Record" }));
  await closed();

  expect(await screen.findByText(`1h15 on ${today()}`)).toBeInTheDocument();
  expect((await getProject(project.id)).activity).toBe("this_week");
});

test("used topics are suggested, most recent first", async () => {
  const project = await addProject({ name: "Maths", type: "learning" });
  await recordTime(project.id, 60, daysAgo(5), "Proba");
  await recordTime(project.id, 60, daysAgo(3), "Stats");
  const { user } = renderApp();
  await openLearning(user, "Maths");
  const dialog = await openRecordTime(user);

  const stats = await dialog.findByRole("button", { name: "Stats" });
  const proba = dialog.getByRole("button", { name: "Proba" });
  expect(stats.compareDocumentPosition(proba) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  await user.click(proba);
  expect(dialog.getByPlaceholderText("Topic (optional)")).toHaveValue("Proba");
});

test("a blank topic is not saved as a topic", async () => {
  const project = await addProject({ name: "Maths", type: "learning" });
  await recordTime(project.id, 60, daysAgo(1), "Proba");
  const { user } = renderApp();
  await openLearning(user, "Maths");
  let dialog = await openRecordTime(user);
  await user.type(dialog.getByPlaceholderText("Topic (optional)"), "   ");
  await user.click(dialog.getByRole("button", { name: "Record" }));
  await closed();
  await screen.findByText(`1h on ${today()}`);

  dialog = await openRecordTime(user);
  await dialog.findByRole("button", { name: "Proba" });
  const fixed = ["−", "+", "Cancel", "Record", "Close", ""];
  const suggestions = dialog
    .getAllByRole("button")
    .map((b) => b.textContent?.trim())
    .filter((t) => !fixed.includes(t ?? ""));
  expect(suggestions).toEqual(["Proba"]);
});

test("hide and show are remembered per project", async () => {
  await addProject({ name: "Maths", type: "learning" });
  await addProject({ name: "Physics", type: "learning" });
  const { user } = renderApp();
  await openLearning(user, "Maths");

  await user.click(taskPane().getByRole("button", { name: "Hide" }));
  expect(screen.getByRole("button", { name: "Show time" })).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Close project" }));
  await openProject(user, "Physics");
  expect(screen.getByRole("button", { name: "Record time" })).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Close project" }));
  await openProject(user, "Maths");
  await user.click(screen.getByRole("button", { name: "Show time" }));
  expect(screen.getByRole("button", { name: "Record time" })).toBeInTheDocument();
});
