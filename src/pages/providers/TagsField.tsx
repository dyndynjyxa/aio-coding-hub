import { X } from "lucide-react";
import { FormField } from "../../ui/FormField";
import { tagBadgeClassName, tagRemoveButtonClassName } from "./providerEditorUtils";

export function TagsField(props: {
  tags: string[];
  setTags: React.Dispatch<React.SetStateAction<string[]>>;
  tagInput: string;
  setTagInput: (value: string) => void;
  saving: boolean;
}) {
  const { tags, setTags, tagInput, setTagInput, saving } = props;

  return (
    <FormField label="标签" hint="按 Enter 添加标签">
      <div className="flex min-h-10 flex-wrap items-center gap-1.5 rounded-lg border border-slate-200 bg-white px-3 shadow-sm dark:border-slate-600 dark:bg-slate-800 dark:shadow-none">
        {tags.map((tag) => (
          <span key={tag} className={tagBadgeClassName(tag)}>
            {tag}
            <button
              type="button"
              onClick={() => setTags((prev) => prev.filter((t) => t !== tag))}
              className={tagRemoveButtonClassName(tag)}
              disabled={saving}
              aria-label={`移除标签 ${tag}`}
            >
              <X className="h-2.5 w-2.5" />
            </button>
          </span>
        ))}
        <input
          type="text"
          value={tagInput}
          onChange={(e) => setTagInput(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key !== "Enter") return;
            e.preventDefault();
            const trimmed = tagInput.trim();
            if (!trimmed) return;
            if (tags.includes(trimmed)) {
              setTagInput("");
              return;
            }
            setTags((prev) => [...prev, trimmed]);
            setTagInput("");
          }}
          placeholder={tags.length === 0 ? "输入标签后按 Enter" : ""}
          className="min-w-[80px] flex-1 border-none bg-transparent text-sm outline-none placeholder:text-slate-400"
          disabled={saving}
        />
      </div>
    </FormField>
  );
}
