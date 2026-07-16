// Usage: 生图适配器公共契约。公共核心字段跨提供商通用，提供商专属参数收进 options 泛型。

export type ImageGenRefImage = { mime: string; b64: string };

export type ImageGenCoreRequest = {
  prompt: string;
  referenceImages: ImageGenRefImage[];
  n: number;
  size: string; // "auto" | "WxH"
};

export type ImageGenUsage = {
  inputTokens?: number;
  outputTokens?: number;
  totalTokens?: number;
  inputTokensDetails?: { textTokens?: number; imageTokens?: number };
};

export type ImageGenResult = {
  images: { mime: string; b64: string }[];
  usage?: ImageGenUsage;
};

export interface ImageGenAdapter<P = Record<string, unknown>> {
  id: string; // "gpt-image"
  label: string; // "GPT Image"
  generate(req: ImageGenCoreRequest & { options: P }): Promise<ImageGenResult>;
}
