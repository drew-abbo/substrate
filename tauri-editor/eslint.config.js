import js from "@eslint/js";
import ts from "typescript-eslint";
import vue from "eslint-plugin-vue";
import vueParser from "vue-eslint-parser";
import globals from "globals";

export default ts.config(
  js.configs.recommended,
  ...ts.configs.recommended,
  ...vue.configs["flat/recommended"],
  {
    // Browser globals (window, document, HTMLElement, etc.)
    languageOptions: {
      globals: {
        ...globals.browser,
      },
    },
  },
  {
    files: ["src/**/*.vue"],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: ts.parser,
        sourceType: "module",
      },
    },
    rules: {
      // Not needed with script setup — components auto-register
      "vue/multi-word-component-names": "off",
      // Style / formatting — let Prettier handle these, not ESLint
      "vue/max-attributes-per-line": "off",
      "vue/html-self-closing": "off",
      "vue/multiline-html-element-content-newline": "off",
      "vue/singleline-html-element-content-newline": "off",
      "vue/html-closing-bracket-spacing": "off",
      "vue/html-closing-bracket-newline": "off",
      "vue/first-attribute-linebreak": "off",
      "vue/attributes-order": "off",
      "vue/no-multi-spaces": "off",
      // Warn rather than error on `any` — useful signal without blocking
      "@typescript-eslint/no-explicit-any": "warn",
    },
  },
  {
    // vite-env.d.ts is a vite-generated ambient declaration — skip strict checks
    files: ["src/vite-env.d.ts"],
    rules: {
      "@typescript-eslint/no-empty-object-type": "off",
      "@typescript-eslint/no-explicit-any": "off",
    },
  },
  {
    ignores: ["node_modules", "dist", "src-tauri"],
  },
);
