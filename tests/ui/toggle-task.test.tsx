import { readFileSync } from "node:fs";
import { waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { addProject, checkbox, gitSubjects, inOrder, openProject, renderApp, struck, taskPane } from "./helpers";

test("checking a task strikes it through in place, writes the file and commits", async () => {
  const project = await addProject({ name: "P", content: "- [ ] alpha\n- [ ] beta\n" });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(checkbox("alpha"));

  await waitFor(() => expect(struck("alpha")).toBe(true));
  expect(struck("beta")).toBe(false);
  expect(inOrder("alpha", "beta")).toBe(true);
  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe("- [x] alpha\n- [ ] beta\n"),
  );
  await waitFor(() => expect(gitSubjects()[0]).toBe(`activity: ${project.id}.md`));
});

test("unchecking a done task clears its strike", async () => {
  const project = await addProject({ name: "P", content: "- [x] alpha\n- [ ] beta\n" });
  const { user } = renderApp();
  await openProject(user, "P");
  expect(struck("alpha")).toBe(true);

  await user.click(checkbox("alpha"));

  await waitFor(() => expect(struck("alpha")).toBe(false));
  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe("- [ ] alpha\n- [ ] beta\n"),
  );
});

test("tasks with the same text toggle independently", async () => {
  const project = await addProject({ name: "P", content: "- [ ] same\n- [ ] same\n" });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(taskPane().getAllByText("same")[1].previousElementSibling as HTMLElement);

  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe("- [ ] same\n- [x] same\n"),
  );
  const same = () => taskPane().getAllByText("same").map((e) => e.style.textDecoration);
  await waitFor(() => expect(same()).toEqual(["none", "line-through"]));
});
