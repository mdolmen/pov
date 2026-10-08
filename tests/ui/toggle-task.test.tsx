import { readFileSync } from "node:fs";
import { screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { addProject, checkbox, doneSection, gitSubjects, openProject, renderApp } from "./helpers";

test("checking a task moves it to Done, writes the file and commits", async () => {
  const project = await addProject({ name: "P", content: "- [ ] alpha\n- [ ] beta\n" });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(checkbox("alpha"));

  await waitFor(() => expect(doneSection()?.getByText("alpha")).toBeInTheDocument());
  expect(screen.getByText("alpha")).toHaveStyle({ textDecoration: "line-through" });
  expect(doneSection()?.queryByText("beta")).toBeNull();
  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe("- [x] alpha\n- [ ] beta\n"),
  );
  await waitFor(() => expect(gitSubjects()[0]).toBe(`activity: ${project.id}.md`));
});

test("unchecking a done task moves it back", async () => {
  const project = await addProject({ name: "P", content: "- [x] alpha\n- [ ] beta\n" });
  const { user } = renderApp();
  await openProject(user, "P");
  expect(doneSection()?.getByText("alpha")).toBeInTheDocument();

  await user.click(checkbox("alpha"));

  await waitFor(() => expect(doneSection()).toBeNull());
  expect(screen.getByText("alpha")).toHaveStyle({ textDecoration: "none" });
  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe("- [ ] alpha\n- [ ] beta\n"),
  );
});

test("tasks with the same text toggle independently", async () => {
  const project = await addProject({ name: "P", content: "- [ ] same\n- [ ] same\n" });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(screen.getAllByText("same")[1].previousElementSibling as HTMLElement);

  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe("- [ ] same\n- [x] same\n"),
  );
  expect(doneSection()?.getAllByText("same")).toHaveLength(1);
});
