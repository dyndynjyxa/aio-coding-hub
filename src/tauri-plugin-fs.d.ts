declare module "@tauri-apps/plugin-fs" {
  export function readTextFile(path: string): Promise<string>;
  export function writeTextFile(path: string, contents: string): Promise<void>;
}
