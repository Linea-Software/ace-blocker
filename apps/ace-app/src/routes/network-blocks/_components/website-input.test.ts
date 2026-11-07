import { describe, expect, it } from "vitest";
import { websiteHostname } from "./website-input";

describe("websiteHostname", () => {
    it.each([
        ["instagram.com", "instagram.com"],
        ["  Instagram.COM  ", "instagram.com"],
        ["https://www.youtube.com/watch?v=123", "www.youtube.com"],
        ["http://reddit.com/r/focus#comments", "reddit.com"],
        ["example.com/path/to/page", "example.com"],
        ["https://example.com:8443/path", "example.com"],
        ["https://bücher.de/search", "xn--bcher-kva.de"],
    ])("extracts hostname from %s", (input, expected) => {
        expect(websiteHostname(input)).toBe(expected);
    });

    it.each([
        "", "example", "a..com", "example_.com", "http://",
        "ftp://example.com", "javascript:alert(1)",
        "https://person@example.com", "example.com other.com",
        "https://example.com@evil.test", "http:/example.com",
        "*.example.com",
    ])("rejects invalid website %s", (input) => {
        expect(websiteHostname(input)).toBeNull();
    });
});
