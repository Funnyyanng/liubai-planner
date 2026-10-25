import ts from "typescript";
import { readFileSync, writeFileSync } from "node:fs";
const configFile = ts.readConfigFile("tsconfig.json", ts.sys.readFile);
const config = ts.parseJsonConfigFileContent(configFile.config, ts.sys, ".");
const host = {
  getScriptFileNames: () => config.fileNames,
  getScriptVersion: () => "0",
  getScriptSnapshot: (f) => {
    try {
      return ts.ScriptSnapshot.fromString(readFileSync(f, "utf8"));
    } catch {
      return undefined;
    }
  },
  getCurrentDirectory: () => process.cwd(),
  getCompilationSettings: () => config.options,
  getDefaultLibFileName: (o) => ts.getDefaultLibFilePath(o),
  fileExists: ts.sys.fileExists,
  readFile: ts.sys.readFile,
  readDirectory: ts.sys.readDirectory,
};
const ls = ts.createLanguageService(host);
for (const file of config.fileNames) {
  for (const change of ls.organizeImports(
    { type: "file", fileName: file },
    {},
    {},
  )) {
    let text = readFileSync(change.fileName, "utf8");
    for (const edit of [...change.textChanges].sort(
      (a, b) => b.span.start - a.span.start,
    ))
      text =
        text.slice(0, edit.span.start) +
        edit.newText +
        text.slice(edit.span.start + edit.span.length);
    writeFileSync(change.fileName, text);
  }
}
