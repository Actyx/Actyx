/* eslint-disable no-undef */
module.exports = {
  root: true,
  parser: "@typescript-eslint/parser",
  plugins: ["@typescript-eslint", "prettier", "react", "react-hooks"],
  extends: [
    "eslint:recommended",
    "prettier",
    "plugin:@typescript-eslint/eslint-recommended",
    "plugin:@typescript-eslint/recommended",
    "plugin:react/recommended",
    "plugin:prettier/recommended",
  ],
  rules: {
    "@typescript-eslint/explicit-function-return-type": "off",
    "@typescript-eslint/no-unused-vars": "off",
    "@typescript-eslint/no-explicit-any": "off",
    "@typescript-eslint/ban-ts-ignore": "off",
    "@typescript-eslint/explicit-module-boundary-types": "off",
    "@typescript-eslint/ban-types": "off",
    "@typescript-eslint/no-var-requires": "off",
    "react/display-name": "off",
    "react/prop-types": "off",
    "react-hooks/rules-of-hooks": "error",
    "react-hooks/exhaustive-deps": "warn",
    "prettier/prettier": ["error", { semi: false }],
  },
  env: {
    browser: true,
    node: true,
    es6: true,
  },
  settings: {
    react: {
      version: "17.0.2",
    },
  },
}
