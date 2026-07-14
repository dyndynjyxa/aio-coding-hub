import type { GatewayHookName, PluginManifest } from "@aio-coding-hub/plugin-sdk";

export type ScaffoldTemplate =
  | "command"
  | "rule"
  | "example:prompt-helper"
  | "example:redactor"
  | "example:response-guard";

export type ScaffoldInput = {
  id: string;
  name: string;
  template: ScaffoldTemplate;
};

export type ScaffoldFiles = Record<string, string>;

export function createPluginScaffold(input: ScaffoldInput): ScaffoldFiles {
  const id = normalizeId(input.id);
  const name = normalizeName(input.name);

  switch (input.template) {
    case "example:prompt-helper":
      return promptHelperExampleTemplate(id, name);
    case "example:redactor":
      return redactorExampleTemplate(id, name);
    case "example:response-guard":
      return responseGuardExampleTemplate(id, name);
    case "command":
    case "rule":
    default:
      return commandTemplate(id, name);
  }
}

function commandTemplate(id: string, name: string): ScaffoldFiles {
  const command = `${id}.hello`;
  const manifest = baseManifest(id, name, "Extension Host command plugin scaffold.");
  manifest.activationEvents = [`onCommand:${command}`];
  manifest.contributes = {
    commands: [
      {
        command,
        title: `Hello from ${name}`,
      },
    ],
  };
  manifest.capabilities = ["commands.execute"];

  return {
    "plugin.json": jsonFile(manifest),
    "dist/extension.js": commandExtensionSource(command, name),
    "README.md": readme(name, id, "This scaffold registers one Extension Host command.", [
      "pnpm --filter create-aio-plugin cli validate --strict .",
      "pnpm --filter create-aio-plugin cli pack .",
    ]),
  };
}

function promptHelperExampleTemplate(id: string, name: string): ScaffoldFiles {
  const hook: GatewayHookName = "gateway.request.afterBodyRead";
  const manifest = gatewayHookManifest(
    id,
    name,
    "Prompt helper example for request body policy hints.",
    [hook]
  );
  const claudeRequestBody = JSON.stringify(
    {
      model: "claude-3-5-sonnet",
      messages: [{ role: "user", content: "Summarize this release note." }],
    },
    null,
    2
  );
  const codexRequestBody = JSON.stringify(
    {
      model: "gpt-5-codex",
      input: [
        {
          role: "user",
          content: [
            {
              type: "input_text",
              text: "CODEX_PROMPT_HELPER: explain this patch.",
            },
          ],
        },
      ],
    },
    null,
    2
  );

  return {
    "plugin.json": jsonFile(manifest),
    "dist/extension.js": `module.exports.activate = function(api) {
  api.gateway.registerHook("${hook}", function(context) {
    const body = String(context && context.request && context.request.body || "");
    if (body.includes("CODEX_PROMPT_HELPER")) {
      return {
        action: "replace",
        requestBody: body.replace(/CODEX_PROMPT_HELPER:?\\s*/g, "Keep answers concise. ")
      };
    }
    if (/claude-[A-Za-z0-9.-]+/i.test(body)) {
      return {
        action: "warn",
        message: "Prompt helper matched a Claude request."
      };
    }
    return { action: "pass" };
  });
};
`,
    "fixtures/claude-request.json": jsonFile({
      request: { body: claudeRequestBody },
    }),
    "fixtures/codex-request.json": jsonFile({
      request: { body: codexRequestBody },
    }),
    "README.md": exampleReadme(
      name,
      id,
      "Adds lightweight prompt guidance to supported request bodies before the gateway sends them upstream."
    ),
  };
}

function redactorExampleTemplate(id: string, name: string): ScaffoldFiles {
  const requestHook: GatewayHookName = "gateway.request.beforeSend";
  const logHook: GatewayHookName = "log.beforePersist";
  const manifest = gatewayHookManifest(
    id,
    name,
    "Redactor example for request bodies and log messages.",
    [requestHook, logHook]
  );

  return {
    "plugin.json": jsonFile(manifest),
    "dist/extension.js": `const secretPattern = /(api_key|token|password)=[A-Za-z0-9_-]+/gi;

module.exports.activate = function(api) {
  api.gateway.registerHook("${requestHook}", function(context) {
    const body = String(context && context.request && context.request.body || "");
    const redacted = body.replace(secretPattern, "[REDACTED]");
    if (redacted !== body) {
      return { action: "replace", requestBody: redacted };
    }
    return { action: "pass" };
  });

  api.gateway.registerHook("${logHook}", function(context) {
    const message = String(context && context.log && context.log.message || "");
    const redacted = message.replace(secretPattern, "[REDACTED]");
    if (redacted !== message) {
      return { action: "replace", logMessage: redacted };
    }
    return { action: "pass" };
  });
};
`,
    "fixtures/request-hit.json": jsonFile({
      request: { body: "POST /v1/chat api_key=sk_live_12345 payload=hello" },
    }),
    "fixtures/request-miss.json": jsonFile({
      request: { body: "POST /v1/chat payload=hello" },
    }),
    "fixtures/log-redact.json": jsonFile({
      log: { message: "provider retry used token=debug_98765" },
    }),
    "README.md": exampleReadme(
      name,
      id,
      "Redacts simple secret-shaped values from request bodies and log messages."
    ),
  };
}

function responseGuardExampleTemplate(id: string, name: string): ScaffoldFiles {
  const hook: GatewayHookName = "gateway.response.after";
  const manifest = gatewayHookManifest(
    id,
    name,
    "Response guard example for review markers in provider output.",
    [hook]
  );

  return {
    "plugin.json": jsonFile(manifest),
    "dist/extension.js": `const riskyPattern = /(delete production|rm -rf|drop database)/i;

module.exports.activate = function(api) {
  api.gateway.registerHook("${hook}", function(context) {
    const body = String(context && context.response && context.response.body || "");
    if (riskyPattern.test(body)) {
      return {
        action: "replace",
        responseBody: body.replace(riskyPattern, "[REVIEW_REQUIRED]")
      };
    }
    return { action: "pass" };
  });
};
`,
    "fixtures/response-warn.json": jsonFile({
      response: { body: "The suggested next step is to run rm -rf /tmp/cache." },
    }),
    "fixtures/response-pass.json": jsonFile({
      response: { body: "The suggested next step is to review the diff and run tests." },
    }),
    "README.md": exampleReadme(
      name,
      id,
      "Marks risky response text for review after the gateway receives the provider response."
    ),
  };
}

function baseManifest(id: string, name: string, description: string): PluginManifest {
  return {
    id,
    name,
    version: "0.1.0",
    apiVersion: "1.0.0",
    main: "dist/extension.js",
    runtime: { kind: "extensionHost", language: "typescript" },
    capabilities: [],
    hostCompatibility: { app: ">=0.60.0 <1.0.0", pluginApi: "^1.0.0" },
    description,
  };
}

function gatewayHookManifest(
  id: string,
  name: string,
  description: string,
  hooks: readonly GatewayHookName[]
): PluginManifest {
  return {
    ...baseManifest(id, name, description),
    activationEvents: hooks.map((hook) => `onGatewayHook:${hook}` as const),
    contributes: {
      gatewayHooks: hooks.map((hook) => ({ name: hook, priority: 100 })),
    },
    capabilities: ["gateway.hooks"],
  };
}

function commandExtensionSource(command: string, name: string): string {
  return `module.exports.activate = function(api) {
  api.commands.registerCommand("${command}", function(args) {
    return {
      ok: true,
      message: "Hello from ${escapeJavaScriptString(name)}",
      args: args || null
    };
  });
};
`;
}

function jsonFile(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function exampleReadme(name: string, id: string, summary: string): string {
  return readme(name, id, summary, [
    "pnpm --filter create-aio-plugin cli validate --strict .",
    "pnpm --filter create-aio-plugin cli pack .",
    "pnpm --filter create-aio-plugin cli publish-check .",
  ]);
}

function readme(name: string, id: string, summary: string, commands: readonly string[]): string {
  const commandList = commands.map((command) => `- \`${command}\``).join("\n");
  return `# ${name}

Plugin ID: \`${id}\`.

${summary}

This scaffold is a development template, not a default installable marketplace package.

## Try it

${commandList}
`;
}

function normalizeId(value: string): string {
  const id = value.trim();
  if (!/^[a-z0-9][a-z0-9-]*(\.[a-z0-9][a-z0-9-]*)+$/.test(id)) {
    throw new Error("PLUGIN_INVALID_ID: expected publisher.plugin-name");
  }
  return id;
}

function normalizeName(value: string): string {
  const name = value.trim();
  if (!name) {
    throw new Error("PLUGIN_INVALID_NAME: plugin name is required");
  }
  return name;
}

function escapeJavaScriptString(value: string): string {
  return value.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}
