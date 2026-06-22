import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "node_modules", "src-tauri", "tests/e2e", "scripts", "coverage"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
);
