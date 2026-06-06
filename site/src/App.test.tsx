import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { apiBaseUrl, download } from "./content";

afterEach(() => {
  cleanup();
});

describe("Nomad Timer landing app", () => {
  it("exposes the Windows executable download link", () => {
    render(<App />);

    const downloadLinks = screen.getAllByRole("link", { name: /Windows|breaktime/i });

    expect(downloadLinks.some((link) => link.getAttribute("href") === download.href)).toBe(true);
  });

  it("shows the production API base URL", () => {
    render(<App />);

    expect(screen.getByText(apiBaseUrl)).toBeInTheDocument();
  });

  it("renders multiple situational pixel cats", () => {
    render(<App />);

    expect(screen.getByRole("img", { name: "집중 고양이" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "기지개 고양이" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "응원 고양이" })).toBeInTheDocument();
  });
});
