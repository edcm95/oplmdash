import "@/i18n";

import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { PlanNotice } from "@/api/generated/PlanNotice";
import i18n from "@/i18n";
import { useNoticeText } from "@/lib/notices";

const notice = (code: string, message: string, phrase?: PlanNotice["phrase"]): PlanNotice => ({
  level: "warning",
  code,
  message,
  ...(phrase ? { phrase } : {}),
});

describe("notice texts", () => {
  afterEach(async () => {
    await i18n.changeLanguage("en");
  });

  it("show the phrase with its names and sizes in English", () => {
    const space = notice("noSpace", "Not enough free space: 5.0 GiB needed, 1.0 MiB available", {
      key: "notices.noSpace",
      params: { needed: 5 * 2 ** 30, free: 2 ** 20 },
    });
    const { result } = renderHook(() => useNoticeText());
    expect(result.current(space)).toBe("Not enough free space: 5.0 GiB needed, 1.0 MiB available");
  });

  it("translate a nested reason", () => {
    const { result } = renderHook(() => useNoticeText());
    const hidden = notice("noStartup", "OPL hides this image: SYSTEM.CNF has no BOOT2 line", {
      key: "notices.noStartup",
      params: { reason: { key: "reasons.noBoot2", params: {} } },
    });
    expect(result.current(hidden)).toBe(
      "OPL hides this image: SYSTEM.CNF has no BOOT2 line (a PS1 disc, a video DVD or a damaged image)",
    );
  });

  it("show the English message of a notice without a phrase", () => {
    const { result } = renderHook(() => useNoticeText());
    expect(result.current(notice("noSpace", "not enough free space: 5 bytes needed, 1 available"))).toBe(
      "not enough free space: 5 bytes needed, 1 available",
    );
  });
});
