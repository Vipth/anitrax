import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/react";
import { isTypingTarget, useHotkeys } from "./hotkeys";

function Harness({
  map,
  opts,
}: {
  map: Record<string, () => void>;
  opts?: Parameters<typeof useHotkeys>[1];
}) {
  useHotkeys(map, opts);
  return (
    <div>
      <input data-testid="field" />
      <button data-testid="btn">x</button>
    </div>
  );
}

function press(key: string, init: KeyboardEventInit = {}, target?: Element) {
  const e = new KeyboardEvent("keydown", { key, bubbles: true, ...init });
  (target ?? window).dispatchEvent(e);
}

describe("isTypingTarget", () => {
  it("detects text inputs and contenteditable", () => {
    const input = document.createElement("input");
    const div = document.createElement("div");
    const editable = document.createElement("div");
    editable.setAttribute("contenteditable", "true");
    expect(isTypingTarget(input)).toBe(true);
    expect(isTypingTarget(editable)).toBe(true);
    expect(isTypingTarget(div)).toBe(false);
    expect(isTypingTarget(null)).toBe(false);
  });
});

describe("useHotkeys", () => {
  it("fires a bare-key handler", () => {
    const fn = vi.fn();
    render(<Harness map={{ j: fn }} />);
    press("j");
    expect(fn).toHaveBeenCalledOnce();
  });

  it("ignores shortcuts while a text field is focused", () => {
    const fn = vi.fn();
    const { getByTestId } = render(<Harness map={{ j: fn }} />);
    const field = getByTestId("field") as HTMLInputElement;
    field.focus();
    press("j", {}, field);
    expect(fn).not.toHaveBeenCalled();
  });

  it("honours allowInInput", () => {
    const fn = vi.fn();
    const { getByTestId } = render(
      <Harness map={{ Escape: fn }} opts={{ allowInInput: ["Escape"] }} />,
    );
    const field = getByTestId("field") as HTMLInputElement;
    field.focus();
    press("Escape", {}, field);
    expect(fn).toHaveBeenCalledOnce();
  });

  it("does not hijack Ctrl/Cmd combos", () => {
    const r = vi.fn();
    render(<Harness map={{ r }} />);
    press("r", { ctrlKey: true });
    press("r", { metaKey: true });
    expect(r).not.toHaveBeenCalled();
    press("r");
    expect(r).toHaveBeenCalledOnce();
  });

  it("matches explicit mod+ combos", () => {
    const save = vi.fn();
    render(<Harness map={{ "mod+s": save }} />);
    press("s", { ctrlKey: true });
    expect(save).toHaveBeenCalledOnce();
  });

  it("distinguishes shift+Arrow from a bare Arrow", () => {
    const tab = vi.fn();
    const grid = vi.fn();
    render(<Harness map={{ "shift+ArrowLeft": tab, ArrowLeft: grid }} />);
    press("ArrowLeft", { shiftKey: true });
    expect(tab).toHaveBeenCalledOnce();
    expect(grid).not.toHaveBeenCalled();
    press("ArrowLeft");
    expect(grid).toHaveBeenCalledOnce();
    expect(tab).toHaveBeenCalledOnce();
  });

  it("can be disabled", () => {
    const fn = vi.fn();
    render(<Harness map={{ j: fn }} opts={{ enabled: false }} />);
    press("j");
    expect(fn).not.toHaveBeenCalled();
  });
});
