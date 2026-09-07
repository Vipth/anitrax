import { describe, expect, it } from "vitest";
import {
  formatScore,
  scoreToTen,
  stripHtml,
  tenToScoreRaw,
} from "./format";

describe("score conversion", () => {
  it("round-trips ten-point scores", () => {
    expect(tenToScoreRaw(8.5)).toBe(85);
    expect(scoreToTen(85)).toBe(8.5);
  });

  it("clamps out-of-range values", () => {
    expect(tenToScoreRaw(-3)).toBe(0);
    expect(tenToScoreRaw(99)).toBe(100);
  });

  it("formats to the user's scale", () => {
    expect(formatScore(0, "POINT_10")).toBe("–");
    expect(formatScore(85, "POINT_100")).toBe("85");
    expect(formatScore(85, "POINT_10")).toBe("9");
    expect(formatScore(85, "POINT_10_DECIMAL")).toBe("8.5");
    expect(formatScore(80, "POINT_5")).toBe("4★");
  });
});

describe("stripHtml", () => {
  it("removes tags and decodes common entities", () => {
    expect(stripHtml("<i>Hello</i> &amp; <b>world</b>")).toBe("Hello & world");
  });
  it("turns <br> into newlines", () => {
    expect(stripHtml("a<br>b")).toBe("a\nb");
  });
  it("handles null", () => {
    expect(stripHtml(null)).toBe("");
  });
});
