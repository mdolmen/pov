import { screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { within } from "@testing-library/react";
import { renderApp } from "./helpers";

async function openSettings(user: ReturnType<typeof renderApp>["user"]) {
  await user.click(await screen.findByRole("button", { name: "Settings" }));
}

test("install CLI installs and shows the result", async () => {
  const { user, calls } = renderApp({ installCli: () => "Installed pov to ~/.local/bin/pov" });
  await openSettings(user);

  await user.click(await screen.findByRole("button", { name: "Install CLI" }));

  expect(await screen.findByText("Installed pov to ~/.local/bin/pov")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "CLI installed" })).toBeDisabled();
  expect(calls.map(([cmd]) => cmd)).toContain("install_cli");
});

test("an installed CLI can't be installed again", async () => {
  const { user } = renderApp({ cliInstalled: true });
  await openSettings(user);

  expect(await screen.findByRole("button", { name: "CLI installed" })).toBeDisabled();
});

test("an install error is shown", async () => {
  const { user } = renderApp({
    installCli: () => {
      throw "pov binary not found";
    },
  });
  await openSettings(user);

  await user.click(await screen.findByRole("button", { name: "Install CLI" }));

  expect(await screen.findByText("pov binary not found")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Install CLI" })).toBeEnabled();
});

test("tools can be added, edited and removed", async () => {
  const { user } = renderApp();
  await openSettings(user);

  await user.type(await screen.findByLabelText("Tool name"), "writer");
  await user.type(screen.getByLabelText("URL"), "http://127.0.0.1:8765/tasks");
  await user.type(screen.getByLabelText("Start command"), "/bin/writer");
  await user.click(screen.getByRole("button", { name: "Add tool" }));

  const list = () => within(screen.getByRole("list", { name: "Tools" }));
  expect(await list().findByText("/bin/writer")).toBeInTheDocument();
  expect(screen.getByLabelText("Tool name")).toHaveValue("");

  await user.click(screen.getByRole("button", { name: "Edit writer" }));
  expect(screen.getByLabelText("Tool name")).toBeDisabled();
  await user.clear(screen.getByLabelText("Start command"));
  await user.type(screen.getByLabelText("Start command"), "/bin/writer2");
  await user.click(screen.getByRole("button", { name: "Save tool" }));
  expect(await list().findByText("/bin/writer2")).toBeInTheDocument();
  expect(list().queryByText("/bin/writer")).not.toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Remove writer" }));
  await waitFor(() => expect(screen.queryByRole("list", { name: "Tools" })).not.toBeInTheDocument());
});

test("a tool's start command must be an absolute path", async () => {
  const { user } = renderApp();
  await openSettings(user);

  await user.type(await screen.findByLabelText("Tool name"), "writer");
  await user.type(screen.getByLabelText("URL"), "http://127.0.0.1:8765/tasks");
  await user.type(screen.getByLabelText("Start command"), "writer");
  await user.click(screen.getByRole("button", { name: "Add tool" }));

  expect(await screen.findByRole("alert")).toHaveTextContent("command must be an absolute path");
  expect(screen.queryByRole("list", { name: "Tools" })).not.toBeInTheDocument();
});
