import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = join(tmpdir(), `aio-plugin-contract-${Date.now()}`);
mkdirSync(join(root, "docs/plugins"), { recursive: true });
mkdirSync(join(root, "docs/plugins/reference"), { recursive: true });
mkdirSync(join(root, "docs/plugins/runtime"), { recursive: true });
mkdirSync(join(root, "packages/plugin-sdk/src"), { recursive: true });
mkdirSync(join(root, "packages/create-aio-plugin/src"), { recursive: true });
mkdirSync(join(root, "src-tauri/src/domain"), { recursive: true });

writeFileSync(
  join(root, "docs/plugins/plugin-api-v1-contract.json"),
  JSON.stringify(
    {
      apiVersion: "1.0.0",
      defaultHookTimeoutMs: 150,
      defaultFailurePolicy: "fail-open",
      activeHooks: ["gateway.request.afterBodyRead"],
      reservedHooks: ["gateway.response.headers"],
      activeMutationFields: ["requestBody"],
      configSchemaTypes: ["object"],
      activePermissions: ["request.body.read"],
      reservedPermissions: ["network.fetch"],
      communityRuntimes: ["declarativeRules"],
      policyGatedRuntimes: ["wasm"],
      officialRuntimes: ["native:privacyFilter"],
    },
    null,
    2
  )
);
writeFileSync(
  join(root, "packages/plugin-sdk/src/index.ts"),
  "gateway.request.afterBodyRead request.body.read declarativeRules"
);
writeFileSync(
  join(root, "packages/create-aio-plugin/src/scaffold.ts"),
  "declarativeRules gateway.request.afterBodyRead request.body.read"
);
writeFileSync(
  join(root, "src-tauri/src/domain/plugins.rs"),
  "gateway.request.afterBodyRead request.body.read declarativeRules"
);
writeFileSync(join(root, "docs/plugin-manifest-v1.md"), "gateway.request.afterBodyRead request.body.read");
writeFileSync(join(root, "docs/plugins/reference/hooks.md"), "gateway.request.afterBodyRead");
writeFileSync(join(root, "docs/plugins/reference/permissions.md"), "request.body.read");
writeFileSync(join(root, "docs/plugins/reference/manifest.md"), "declarativeRules wasm native privacyFilter");
writeFileSync(join(root, "docs/plugins/runtime/wasm.md"), "wasm PLUGIN_RUNTIME_DISABLED");

const result = spawnSync("node", ["scripts/check-plugin-api-contract.mjs"], {
  cwd: process.cwd(),
  env: { ...process.env, AIO_PLUGIN_CONTRACT_TEST_ROOT: root },
  encoding: "utf8",
});

if (result.status === 0 || !result.stderr.includes("gateway.response.headers")) {
  throw new Error(
    `expected structural contract failure, got status ${result.status}\n${result.stderr}`
  );
}
