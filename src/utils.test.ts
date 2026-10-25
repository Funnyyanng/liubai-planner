import { describe, expect, it } from "vitest";
import {
  clock,
  datetimeInput,
  zonedMinute,
  dayKey,
  inputMinute,
} from "./utils";
describe("time boundaries", () => {
  it("renders one instant in the configured timezone across dates", () => {
    const min = Date.parse("2026-09-06T20:00:00Z") / 60000;
    expect(dayKey(min, "Asia/Shanghai")).toBe("2026-09-07");
    expect(clock(min, "Asia/Shanghai")).toBe("04:00");
    expect(dayKey(min, "America/New_York")).toBe("2026-09-06");
  });
  it("local editable time round trips without changing the instant", () => {
    const min = Date.parse("2026-09-06T20:37:00Z") / 60000;
    expect(inputMinute(datetimeInput(min))).toBe(min);
  });
});

it("dragging uses planning timezone instead of computer timezone", () => {
  expect(zonedMinute("2026-09-07", 9, "Asia/Shanghai")).toBe(
    Date.parse("2026-09-07T01:00:00Z") / 60000,
  );
  expect(() => zonedMinute("2026-03-08", 2, "America/New_York")).toThrow();
});
