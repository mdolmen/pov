import { screen } from "@testing-library/react";
import { expect, test } from "vitest";
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
