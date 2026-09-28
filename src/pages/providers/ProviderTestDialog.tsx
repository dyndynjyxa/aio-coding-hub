// Usage: Collects the model + prompt used by the provider availability probe.
// Only gathers input — the probe call, toasts and loading state stay in the view model.

import { useEffect, useId, useState } from "react";
import type { ProviderSummary } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import {
  DEFAULT_PROBE_PROMPT,
  defaultProbeModel,
  probeModelCandidates,
} from "./providerProbeDefaults";

export function ProviderTestDialog({
  provider,
  testing,
  onClose,
  onConfirm,
}: {
  provider: ProviderSummary | null;
  testing: boolean;
  onClose: () => void;
  onConfirm: (input: { model: string; prompt: string }) => void;
}) {
  const candidateListId = useId();
  const [model, setModel] = useState("");
  const [prompt, setPrompt] = useState(DEFAULT_PROBE_PROMPT);

  // Reseed per provider; the dialog intentionally keeps no draft between openings.
  useEffect(() => {
    if (!provider) return;
    setModel(defaultProbeModel(provider.model_policy));
    setPrompt(DEFAULT_PROBE_PROMPT);
  }, [provider]);

  const candidates = provider ? probeModelCandidates(provider.model_policy) : [];

  return (
    <Dialog
      open={!!provider}
      onOpenChange={(nextOpen) => {
        if (!nextOpen && !testing) onClose();
      }}
      title="测试供应商可用性"
      description={provider ? `将测试：${provider.name}` : undefined}
      className="max-w-lg"
    >
      <div className="space-y-3">
        <datalist id={candidateListId}>
          {candidates.map((candidate) => (
            <option key={candidate} value={candidate} />
          ))}
        </datalist>

        <FormField label="模型" hint="留空则使用该供应商已配置的模型">
          {(id) => (
            <Input
              id={id}
              list={candidateListId}
              value={model}
              onChange={(event) => setModel(event.currentTarget.value)}
              placeholder="例如: deepseek-v4-flash"
              disabled={testing}
            />
          )}
        </FormField>

        <FormField label="提示词" hint={`留空则使用默认 ${DEFAULT_PROBE_PROMPT}`}>
          {(id) => (
            <Input
              id={id}
              value={prompt}
              onChange={(event) => setPrompt(event.currentTarget.value)}
              placeholder={DEFAULT_PROBE_PROMPT}
              disabled={testing}
            />
          )}
        </FormField>

        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button onClick={onClose} variant="secondary" disabled={testing}>
            取消
          </Button>
          <Button onClick={() => onConfirm({ model, prompt })} variant="primary" disabled={testing}>
            {testing ? "测试中…" : "开始测试"}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
