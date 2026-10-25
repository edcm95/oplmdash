import "@/i18n";

import { afterEach, describe, expect, it } from "vitest";

import type { Phrase } from "@/api/generated/Phrase";
import { RpcError } from "@/api/transport";
import i18n from "@/i18n";
import { errorText, phraseText } from "@/lib/phrase";

const phrase = (key: string, params: Phrase["params"] = {}): Phrase => ({ key, params });

describe("phrases from the core", () => {
  afterEach(async () => {
    await i18n.changeLanguage("en");
  });

  it("pick the plural form and match the core's English", () => {
    expect(phraseText(phrase("titles.importGames", { count: 1 }), "")).toBe("Import 1 game");
    expect(phraseText(phrase("titles.importGames", { count: 3 }), "")).toBe("Import 3 games");
    expect(phraseText(phrase("titles.importGames", { count: 25 }), "")).toBe("Import 25 games");
  });

  it("translate nested phrases and format sizes", () => {
    const undo = phrase("titles.recoveryUndo", {
      title: phrase("titles.deleteGame", { title: "Retail Game" }),
    });
    expect(phraseText(undo, "")).toBe("Undo: Delete Retail Game");
    const space = phrase("errors.noSpace", { needed: 5 * 2 ** 30, free: 2 ** 20 });
    expect(phraseText(space, "")).toBe("Not enough free space: 5.0 GiB needed, 1.0 MiB available");
  });

  it("fall back to the core's English for a key the catalogue lacks", () => {
    expect(phraseText(phrase("titles.fromANewerCore"), "Do something new")).toBe("Do something new");
    const nested = phrase("errors.planBlocked", { reason: phrase("notices.fromANewerCore") });
    expect(phraseText(nested, "The plan cannot run. Something new")).toBe(
      "The plan cannot run. Something new",
    );
    expect(phraseText(null, "Import 2 games")).toBe("Import 2 games");
  });

  it("show API errors in English", () => {
    const error = new RpcError({
      code: "NotFound",
      message: "Plan 7 expired or already ran",
      phrase: phrase("errors.planExpired", { id: "7" }),
    });
    expect(errorText(error)).toBe("Plan 7 expired or already ran");
    expect(errorText({ code: "Internal", message: "boom" })).toBe("boom");
    expect(errorText(new Error("plain"))).toBe("plain");
  });
});
