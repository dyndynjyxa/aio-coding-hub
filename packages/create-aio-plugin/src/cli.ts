import { runCreateAioPluginCli } from "./devtools";

process.exit(runCreateAioPluginCli(process.argv.slice(2), process.cwd()));
