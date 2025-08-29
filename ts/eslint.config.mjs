import globals from "globals";
import tseslint from "typescript-eslint";
import eslint from '@eslint/js';
import _import from "eslint-plugin-import";

export default tseslint.config({
  extends: [eslint.configs.recommended, tseslint.configs.recommended],
  plugins: {
    import: _import,
    "@typescript-eslint": tseslint.plugin,
  },
  languageOptions: {
    globals: {
      ...globals.browser,
    },

    parser: tseslint.parser,
    ecmaVersion: 12,
    sourceType: "module",

    parserOptions: {
      project: "./tsconfig.json",
    },
  },
  rules: {
    semi: ["error", "always"],

    quotes: [
      "error",
      "double",
      {
        avoidEscape: true,
      },
    ],

    "no-console": [
      "warn",
      {
        allow: ["warn", "error"],
      },
    ],

    "arrow-parens": ["error", "always"],
    "no-param-reassign": "error",
    "no-sequences": "error",
    "object-shorthand": "error",
    "prefer-const": "error",
    "prefer-object-spread": "error",
    "import/no-default-export": "error",

    "import/no-internal-modules": [
      "error",
      {
        allow: ["unfetch/polyfill/index", "promise-polyfill/dist/polyfill"],
      },
    ],

    "import/no-extraneous-dependencies": "error",

    "@typescript-eslint/no-unused-vars": [
      "warn",
      {
        argsIgnorePattern: "^_",
      },
    ],

    "@typescript-eslint/array-type": [
      "error",
      {
        default: "array-simple",
      },
    ],

    "@typescript-eslint/prefer-readonly": "error",
    "@typescript-eslint/no-dynamic-delete": "error",
    "@typescript-eslint/no-require-imports": "error",

    "@typescript-eslint/strict-boolean-expressions": [
      "error",
      {
        allowString: false,
        allowNumber: false,
        allowNullableObject: false,
      },
    ],
  }
});

