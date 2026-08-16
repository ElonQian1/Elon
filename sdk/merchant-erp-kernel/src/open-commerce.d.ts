import type { MerchantErpKernel, OpenCommerceProvider } from "./index.js";

export function createOpenCommerceProvider(kernel: MerchantErpKernel): OpenCommerceProvider;
export function createMerchantRuntimeBinding(provider: OpenCommerceProvider): {
  readonly schema: "yilong.erp.merchant_runtime_binding.v1";
  readonly merchantId: string;
  readonly capabilities: readonly {
    key: string;
    access: "public" | "authorized";
    action: boolean;
    input_schema: Record<string, unknown>;
  }[];
  readonly handlers: Readonly<Record<
    string,
    (
      input: Record<string, unknown>,
      context: { merchantId: string; idempotencyKey: string },
    ) => Promise<unknown>
  >>;
};
