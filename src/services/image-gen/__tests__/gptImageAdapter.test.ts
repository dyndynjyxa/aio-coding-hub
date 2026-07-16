import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  buildEditsParts,
  buildGenerationsBody,
  buildRequestUrlPreview,
  EDITS_PATH,
  estimateGptImageCostUsd,
  extFromMime,
  extractApiErrorMessage,
  GENERATIONS_PATH,
  gptImageAdapter,
  normalizeBaseUrl,
  outputMimeFor,
  parseImagesResponse,
  parseUsage,
  type GptImageOptions,
  type GptImageRequest,
} from "../gptImageAdapter";
import {
  IMAGE_GEN_ADAPTER_ID,
  imageGenFetchImage,
  imageGenPostJson,
  imageGenPostMultipart,
} from "../service";

vi.mock("../service", async () => {
  const actual = await vi.importActual<typeof import("../service")>("../service");
  return {
    ...actual,
    imageGenPostJson: vi.fn(),
    imageGenPostMultipart: vi.fn(),
    imageGenFetchImage: vi.fn(),
  };
});

function makeRequest(
  overrides: Partial<Omit<GptImageRequest, "options">> = {},
  optionOverrides: Partial<GptImageOptions> = {}
): GptImageRequest {
  return {
    prompt: "一只猫",
    referenceImages: [],
    n: 1,
    size: "auto",
    ...overrides,
    options: {
      model: "gpt-image-2",
      quality: "auto",
      outputFormat: "png",
      outputCompression: null,
      moderation: "auto",
      ...optionOverrides,
    },
  };
}

describe("services/image-gen/gptImageAdapter", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("normalizeBaseUrl", () => {
    it("returns empty string for blank input", () => {
      expect(normalizeBaseUrl("   ")).toBe("");
    });

    it("adds https:// and /v1 for a bare domain", () => {
      expect(normalizeBaseUrl(" api.example.com ")).toBe("https://api.example.com/v1");
    });

    it("appends /v1 when path is empty even with trailing slash", () => {
      expect(normalizeBaseUrl("https://api.example.com/")).toBe("https://api.example.com/v1");
    });

    it("keeps an existing /v1 path untouched", () => {
      expect(normalizeBaseUrl("https://api.example.com/v1")).toBe("https://api.example.com/v1");
    });

    it("does not append /v1 for a custom path", () => {
      expect(normalizeBaseUrl("https://host.example.com/openai")).toBe(
        "https://host.example.com/openai"
      );
    });

    it("keeps http scheme for local debugging", () => {
      expect(normalizeBaseUrl("http://127.0.0.1:8080")).toBe("http://127.0.0.1:8080/v1");
    });
  });

  describe("buildRequestUrlPreview", () => {
    it("returns empty string for blank base url", () => {
      expect(buildRequestUrlPreview("", GENERATIONS_PATH)).toBe("");
    });

    it("dedupes /v1 between base and path", () => {
      expect(buildRequestUrlPreview("api.example.com", GENERATIONS_PATH)).toBe(
        "https://api.example.com/v1/images/generations"
      );
    });

    it("appends the full path for custom-path base urls", () => {
      expect(buildRequestUrlPreview("https://host.example.com/openai", GENERATIONS_PATH)).toBe(
        "https://host.example.com/openai/v1/images/generations"
      );
    });
  });

  describe("buildGenerationsBody", () => {
    it("always sends model/prompt/size/output_format/moderation/quality and never forbidden keys", () => {
      const body = buildGenerationsBody(makeRequest()) as Record<string, unknown>;
      expect(body).toEqual({
        model: "gpt-image-2",
        prompt: "一只猫",
        size: "auto",
        output_format: "png",
        moderation: "auto",
        quality: "auto",
      });
      for (const forbidden of ["background", "input_fidelity", "response_format", "user"]) {
        expect(body).not.toHaveProperty(forbidden);
      }
    });

    it("sends n only when n > 1", () => {
      const single = buildGenerationsBody(makeRequest({ n: 1 })) as Record<string, unknown>;
      const multi = buildGenerationsBody(makeRequest({ n: 3 })) as Record<string, unknown>;
      expect(single).not.toHaveProperty("n");
      expect(multi.n).toBe(3);
    });

    it("sends output_compression only for non-png formats with a value", () => {
      const jpeg = buildGenerationsBody(
        makeRequest({}, { outputFormat: "jpeg", outputCompression: 80 })
      ) as Record<string, unknown>;
      const jpegNull = buildGenerationsBody(
        makeRequest({}, { outputFormat: "jpeg", outputCompression: null })
      ) as Record<string, unknown>;
      const png = buildGenerationsBody(
        makeRequest({}, { outputFormat: "png", outputCompression: 80 })
      ) as Record<string, unknown>;
      expect(jpeg.output_compression).toBe(80);
      expect(jpegNull).not.toHaveProperty("output_compression");
      expect(png).not.toHaveProperty("output_compression");
    });
  });

  describe("buildEditsParts", () => {
    it("builds fields with the same conditional semantics", () => {
      const { fields } = buildEditsParts(
        makeRequest(
          { n: 2, referenceImages: [{ mime: "image/png", b64: "AAA" }] },
          { outputFormat: "webp", outputCompression: 66 }
        )
      );
      expect(fields).toEqual([
        ["model", "gpt-image-2"],
        ["prompt", "一只猫"],
        ["size", "auto"],
        ["output_format", "webp"],
        ["moderation", "auto"],
        ["quality", "auto"],
        ["n", "2"],
        ["output_compression", "66"],
      ]);
    });

    it("omits n and output_compression on the base case", () => {
      const { fields } = buildEditsParts(
        makeRequest({ referenceImages: [{ mime: "image/png", b64: "AAA" }] })
      );
      const keys = fields.map(([key]) => key);
      expect(keys).not.toContain("n");
      expect(keys).not.toContain("output_compression");
    });

    it("names files image[] with input-{i+1}.{ext}", () => {
      const { files } = buildEditsParts(
        makeRequest({
          referenceImages: [
            { mime: "image/png", b64: "AAA" },
            { mime: "image/jpeg", b64: "BBB" },
          ],
        })
      );
      expect(files).toEqual([
        { field: "image[]", filename: "input-1.png", mime: "image/png", dataB64: "AAA" },
        { field: "image[]", filename: "input-2.jpeg", mime: "image/jpeg", dataB64: "BBB" },
      ]);
    });
  });

  describe("extFromMime / outputMimeFor", () => {
    it("maps known mimes and falls back for unknown values", () => {
      expect(extFromMime("image/png")).toBe("png");
      expect(extFromMime("image/webp")).toBe("webp");
      expect(extFromMime("image/x-custom")).toBe("x-custom");
      expect(extFromMime("weird")).toBe("png");
    });

    it("derives the output mime from the format", () => {
      expect(outputMimeFor("jpeg")).toBe("image/jpeg");
    });
  });

  describe("extractApiErrorMessage", () => {
    it("prefers error.message", () => {
      expect(extractApiErrorMessage('{"error":{"message":"bad key"}}', 401)).toBe("bad key");
    });

    it("falls back to detail string", () => {
      expect(extractApiErrorMessage('{"detail":"denied"}', 403)).toBe("denied");
    });

    it("joins detail arrays", () => {
      expect(extractApiErrorMessage('{"detail":["a",{"msg":"b"}]}', 422)).toBe('a; {"msg":"b"}');
    });

    it("falls back to error string then message", () => {
      expect(extractApiErrorMessage('{"error":"plain"}', 400)).toBe("plain");
      expect(extractApiErrorMessage('{"message":"msg"}', 400)).toBe("msg");
    });

    it("uses HTTP status with raw text for non-JSON bodies", () => {
      expect(extractApiErrorMessage("gateway exploded", 500)).toBe("HTTP 500: gateway exploded");
    });

    it("uses bare HTTP status for empty bodies", () => {
      expect(extractApiErrorMessage("", 502)).toBe("HTTP 502");
    });
  });

  describe("parseUsage", () => {
    it("extracts token counts", () => {
      expect(parseUsage({ input_tokens: 1, output_tokens: 2, total_tokens: 3 })).toEqual({
        inputTokens: 1,
        outputTokens: 2,
        totalTokens: 3,
      });
    });

    it("tolerates partial usage", () => {
      expect(parseUsage({ total_tokens: 9 })).toEqual({ totalTokens: 9 });
    });

    it("returns undefined for missing or invalid usage", () => {
      expect(parseUsage(undefined)).toBeUndefined();
      expect(parseUsage("nope")).toBeUndefined();
      expect(parseUsage({})).toBeUndefined();
    });

    it("extracts input token details", () => {
      expect(
        parseUsage({
          input_tokens: 10,
          input_tokens_details: { text_tokens: 6, image_tokens: 4 },
        })
      ).toEqual({ inputTokens: 10, inputTokensDetails: { textTokens: 6, imageTokens: 4 } });
    });

    it("tolerates partial or invalid input token details", () => {
      expect(parseUsage({ input_tokens: 10, input_tokens_details: { text_tokens: 6 } })).toEqual({
        inputTokens: 10,
        inputTokensDetails: { textTokens: 6 },
      });
      // 非对象 / 空对象明细被忽略，不产生 inputTokensDetails 字段。
      expect(parseUsage({ input_tokens: 10, input_tokens_details: "nope" })).toEqual({
        inputTokens: 10,
      });
      expect(parseUsage({ input_tokens: 10, input_tokens_details: { text_tokens: "x" } })).toEqual({
        inputTokens: 10,
      });
    });
  });

  describe("estimateGptImageCostUsd", () => {
    it("prices with input details: text*$5 + image*$8 + output*$30 per 1M", () => {
      expect(
        estimateGptImageCostUsd({
          inputTokens: 100,
          outputTokens: 1000,
          inputTokensDetails: { textTokens: 60, imageTokens: 40 },
        })
      ).toBeCloseTo((60 * 5 + 40 * 8 + 1000 * 30) / 1_000_000, 10);
    });

    it("falls back to input*$5 + output*$30 without details", () => {
      expect(estimateGptImageCostUsd({ inputTokens: 100, outputTokens: 1000 })).toBeCloseTo(
        (100 * 5 + 1000 * 30) / 1_000_000,
        10
      );
      // 明细缺项容忍：只有 textTokens 也走明细分支。
      expect(estimateGptImageCostUsd({ inputTokensDetails: { textTokens: 200 } })).toBeCloseTo(
        (200 * 5) / 1_000_000,
        10
      );
    });

    it("returns null for missing usage or no billable fields", () => {
      expect(estimateGptImageCostUsd(undefined)).toBeNull();
      expect(estimateGptImageCostUsd({})).toBeNull();
      // 只有 totalTokens 无法拆分计价。
      expect(estimateGptImageCostUsd({ totalTokens: 100 })).toBeNull();
    });
  });

  describe("parseImagesResponse", () => {
    const noFetch = vi.fn();

    it("extracts b64_json entries with usage", async () => {
      const body = JSON.stringify({
        data: [{ b64_json: "AAA" }, { b64_json: "BBB" }],
        usage: { total_tokens: 42 },
      });
      const result = await parseImagesResponse(body, "image/png", noFetch);
      expect(result.images).toEqual([
        { mime: "image/png", b64: "AAA" },
        { mime: "image/png", b64: "BBB" },
      ]);
      expect(result.usage).toEqual({ totalTokens: 42 });
      expect(noFetch).not.toHaveBeenCalled();
    });

    it("falls back to url download for url-only entries", async () => {
      const fetchImage = vi.fn().mockResolvedValue({ mime: "image/jpeg", b64: "FETCHED" } as never);
      const body = JSON.stringify({ data: [{ url: "https://cdn.example.com/a.jpg" }] });
      const result = await parseImagesResponse(body, "image/png", fetchImage);
      expect(fetchImage).toHaveBeenCalledWith("https://cdn.example.com/a.jpg");
      expect(result.images).toEqual([{ mime: "image/jpeg", b64: "FETCHED" }]);
      expect(result.usage).toBeUndefined();
    });

    it("throws for invalid JSON", async () => {
      await expect(parseImagesResponse("not json", "image/png", noFetch)).rejects.toThrow(
        "响应不是有效的 JSON"
      );
    });

    it("throws when data is missing or empty", async () => {
      await expect(parseImagesResponse("{}", "image/png", noFetch)).rejects.toThrow("data 为空");
      await expect(parseImagesResponse('{"data":[]}', "image/png", noFetch)).rejects.toThrow(
        "data 为空"
      );
    });

    it("throws when no entry is parseable", async () => {
      await expect(parseImagesResponse('{"data":[{}]}', "image/png", noFetch)).rejects.toThrow(
        "缺少 b64_json / url"
      );
    });
  });

  describe("gptImageAdapter.generate", () => {
    it("posts JSON to generations without reference images", async () => {
      vi.mocked(imageGenPostJson).mockResolvedValue({
        status: 200,
        bodyText: JSON.stringify({ data: [{ b64_json: "AAA" }] }),
      });
      const request = makeRequest();
      const result = await gptImageAdapter.generate(request);
      expect(imageGenPostJson).toHaveBeenCalledWith(
        IMAGE_GEN_ADAPTER_ID,
        GENERATIONS_PATH,
        buildGenerationsBody(request)
      );
      expect(imageGenPostMultipart).not.toHaveBeenCalled();
      expect(result.images).toEqual([{ mime: "image/png", b64: "AAA" }]);
    });

    it("posts multipart to edits with reference images", async () => {
      vi.mocked(imageGenPostMultipart).mockResolvedValue({
        status: 200,
        bodyText: JSON.stringify({ data: [{ b64_json: "CCC" }] }),
      });
      const request = makeRequest({ referenceImages: [{ mime: "image/png", b64: "AAA" }] });
      const { fields, files } = buildEditsParts(request);
      await gptImageAdapter.generate(request);
      expect(imageGenPostMultipart).toHaveBeenCalledWith(
        IMAGE_GEN_ADAPTER_ID,
        EDITS_PATH,
        fields,
        files
      );
      expect(imageGenPostJson).not.toHaveBeenCalled();
    });

    it("throws the extracted error message for non-2xx responses", async () => {
      vi.mocked(imageGenPostJson).mockResolvedValue({
        status: 429,
        bodyText: '{"error":{"message":"rate limited"}}',
      });
      await expect(gptImageAdapter.generate(makeRequest())).rejects.toThrow("rate limited");
    });

    it("routes url-only entries through imageGenFetchImage", async () => {
      vi.mocked(imageGenPostJson).mockResolvedValue({
        status: 200,
        bodyText: JSON.stringify({ data: [{ url: "https://cdn.example.com/a.png" }] }),
      });
      vi.mocked(imageGenFetchImage).mockResolvedValue({ mime: "image/png", dataB64: "DL" });
      const result = await gptImageAdapter.generate(makeRequest());
      expect(imageGenFetchImage).toHaveBeenCalledWith("https://cdn.example.com/a.png");
      expect(result.images).toEqual([{ mime: "image/png", b64: "DL" }]);
    });

    it("resolves mixed b64_json and url entries in order within one response", async () => {
      vi.mocked(imageGenPostJson).mockResolvedValue({
        status: 200,
        bodyText: JSON.stringify({
          data: [{ b64_json: "AAA" }, { url: "https://cdn.example.com/b.jpg" }],
        }),
      });
      vi.mocked(imageGenFetchImage).mockResolvedValue({ mime: "image/jpeg", dataB64: "BBB" });

      const result = await gptImageAdapter.generate(makeRequest());

      expect(imageGenFetchImage).toHaveBeenCalledTimes(1);
      expect(imageGenFetchImage).toHaveBeenCalledWith("https://cdn.example.com/b.jpg");
      expect(result.images).toEqual([
        { mime: "image/png", b64: "AAA" },
        { mime: "image/jpeg", b64: "BBB" },
      ]);
    });

    it("rejects the whole generate with a readable error when the url fallback download fails", async () => {
      vi.mocked(imageGenPostJson).mockResolvedValue({
        status: 200,
        bodyText: JSON.stringify({ data: [{ url: "https://cdn.example.com/a.png" }] }),
      });
      vi.mocked(imageGenFetchImage).mockRejectedValue(new Error("下载生成图片失败"));

      await expect(gptImageAdapter.generate(makeRequest())).rejects.toThrow("下载生成图片失败");
    });
  });
});
