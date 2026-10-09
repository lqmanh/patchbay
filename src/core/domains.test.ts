import { describe, expect, test } from "bun:test";
import { createRule, normalizeDomain, regexError } from "./domains";

describe("normalizeDomain", () => {
  test("normalizes urls, ports and case", () => {
    expect(normalizeDomain("  Reddit.COM ")).toBe("reddit.com");
    expect(normalizeDomain("https://www.reddit.com/r/programming")).toBe("reddit.com");
    expect(normalizeDomain("sub.medium.com:8443")).toBe("sub.medium.com");
    expect(normalizeDomain("example.org.")).toBe("example.org");
  });

  test("rejects invalid hosts", () => {
    expect(normalizeDomain("")).toBeNull();
    expect(normalizeDomain("single-label")).toBeNull();
    expect(normalizeDomain("has space.com")).toBeNull();
    expect(normalizeDomain("https://")).toBeNull();
  });
});

describe("regexError", () => {
  test("accepts RE2-safe patterns", () => {
    expect(regexError("^(.*\\.)?example\\.com$")).toBeNull();
  });

  test("rejects invalid or unsupported patterns", () => {
    expect(regexError("(")).not.toBeNull();
    expect(regexError("(?=foo)")).not.toBeNull();
    expect(regexError("(a)\\1")).not.toBeNull();
    expect(regexError("")).not.toBeNull();
  });
});

describe("createRule", () => {
  test("creates domain rules from urls", () => {
    const rule = createRule("https://www.medium.com/topic");
    expect(rule).toMatchObject({ value: "medium.com", kind: "domain", enabled: true });
    expect(rule?.id).toBeTruthy();
  });

  test("creates regex rules only when valid", () => {
    expect(createRule("^(.*\\.)?files\\.example\\.com$", "regex")).toMatchObject({
      kind: "regex",
    });
    expect(createRule("(?=bad)", "regex")).toBeNull();
    expect(createRule("not a domain")).toBeNull();
  });
});
