import { screen, waitFor, within } from "@testing-library/react";
import { expect, test } from "vitest";
import {
  addProject,
  checkbox,
  nextButton,
  openProject,
  projectTiles,
  queueButton,
  queueTask,
  renderApp,
  struck,
  taskHash,
  taskPane,
  tileName,
} from "./helpers";

/** The band on a task's row: "next", "queue" or null. */
const band = (text: string) => {
  const row = taskPane().getByText(text).closest("[data-task]") as HTMLElement;
  return row.style.boxShadow.match(/--pov-(next|queue)/)?.[1] ?? null;
};

const TODO = "- [ ] alpha\n- [ ] beta\n- [ ] gamma\n";

test("Queue bands a task in amber, Queued unqueues it", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  expect(queueButton("beta")).toHaveTextContent("Queue");

  await user.click(queueButton("beta"));
  expect(band("beta")).toBe("queue");
  expect(queueButton("beta")).toHaveTextContent("Queued");

  await user.click(queueButton("beta"));
  expect(band("beta")).toBe(null);
});

test("Mark next moves a queued task up, and it stays next", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  await user.click(queueButton("beta"));

  await user.click(nextButton("beta"));
  expect(band("beta")).toBe("next");
  expect(taskPane().queryByRole("button", { name: "Queued" })).not.toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Close project" }));
  await openProject(user, "P");
  expect(band("beta")).toBe("next");
});

test("double-clicking a queued task marks it next", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  await user.click(queueButton("gamma"));

  await user.dblClick(taskPane().getByText("gamma"));

  expect(band("gamma")).toBe("next");
});

test("the tile shows a badge with the queued count", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  const tile = () => projectTiles().getByRole("button", { name: tileName("P") });
  expect(within(tile()).queryByLabelText(/queued$/)).not.toBeInTheDocument();

  await user.click(queueButton("alpha"));
  await user.click(queueButton("beta"));
  await user.click(nextButton("gamma"));
  expect(await projectTiles().findByRole("button", { name: /^P1 next of 3 open/ })).toBeInTheDocument();
  expect(await within(tile()).findByLabelText("2 queued")).toHaveTextContent("2");

  await user.click(queueButton("alpha"));
  expect(await within(tile()).findByLabelText("1 queued")).toBeInTheDocument();
});

test("queued tasks stay out of Next up", async () => {
  const p = await addProject({ name: "P", content: "## M\n- [ ] alpha\n" });
  await queueTask(p.id, await taskHash(p.id, "alpha"));
  renderApp();

  expect(await screen.findByText("Nothing marked next")).toBeInTheDocument();
});

test("checking a queued task strikes it and drops its mark", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  await user.click(queueButton("beta"));

  await user.click(checkbox("beta"));

  await waitFor(() => expect(struck("beta")).toBe(true));
  expect(band("beta")).toBe(null);
  await waitFor(() =>
    expect(projectTiles().getByRole("button", { name: /^P2 open/ })).toBeInTheDocument(),
  );
});

test("Queued shows only the queued tasks, under their milestones", async () => {
  await addProject({
    name: "P",
    content: "## One\n- [ ] alpha\n- [x] beta\n## Two\n- [ ] gamma\n- [ ] delta\n",
  });
  const { user } = renderApp();
  await openProject(user, "P");
  expect(taskPane().queryByRole("button", { name: "Queued" })).not.toBeInTheDocument();
  await user.click(queueButton("delta"));
  await user.click(nextButton("alpha"));

  const filter = screen.getByRole("button", { name: "Queued", pressed: false });
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

test("unqueuing the last queued task turns the filter off", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  await user.click(queueButton("beta"));
  await user.click(screen.getByRole("button", { name: "Queued", pressed: false }));

  await user.click(queueButton("beta"));

  expect(screen.queryByRole("button", { name: "Queued", pressed: true })).not.toBeInTheDocument();
  expect(taskPane().getByText("alpha")).toBeInTheDocument();
});
