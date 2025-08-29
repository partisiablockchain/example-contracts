module.exports = {
  globals: {
    window: true,
    document: true,
  },
  roots: [
    "<rootDir>/src",
  ],
  testEnvironment: "node",
  transform: {
    "^.+\\.tsx?$": "ts-jest",
  },
  testRegex: "(/__tests__/.*|(\\.|/)(test|spec))\\.tsx?$",
  moduleFileExtensions: [
    "ts",
    "tsx",
    "js",
    "jsx",
    "json",
    "node",
  ],
  moduleDirectories: [
    "src/main",
    "src/test",
    "node_modules",
  ],
  testEnvironmentOptions: {
    url: "http://localhost/"
  },
  collectCoverage: true,
  collectCoverageFrom: [
    "src/main/**",
  ],
  coverageReporters: [
    "lcov",
  ],
  coverageThreshold: {
    global: {
      branches: 100,
      functions: 100,
      lines: 100,
      statements: 100,
    },
  },
  coverageDirectory: "target/",
  automock: false,
};
