// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

import type { JSONSchemaForCircleCIConfigurationFiles } from "@fensak-io/senc-schemastore-ciconfig";

import {
  addSSHKeyStep,
  addRestoreCacheStep,
  addSaveCacheStep,
  dockerCfgRustWithNodeImg,
  executors,
  getBuildUnixJob,
  getBuildWindowsJob,
  outPrefix,
} from "./common.ts";

const filterMainBranches = {
  branches: {
    only: ["main", "release"],
  },
};
const filterReleaseBranch = {
  branches: {
    only: ["release"],
  },
};

const cfg: JSONSchemaForCircleCIConfigurationFiles = {
  version: 2.1,
  workflows: {
    lint_test_release: {
      jobs: [
        "lint_test",
        {
          build_linux_amd64: {
            filters: filterMainBranches,
          },
        },
        {
          build_linux_arm64: {
            filters: filterMainBranches,
          },
        },
        {
          build_windows_amd64: {
            filters: filterMainBranches,
          },
        },
        {
          build_darwin_arm64: {
            filters: filterMainBranches,
          },
        },
        {
          release: {
            context: "Fensak CI/CD",
            requires: [
              "build_linux_amd64",
              "build_linux_arm64",
              "build_windows_amd64",
              "build_darwin_arm64",
            ],
            filters: filterReleaseBranch,
          },
        },
      ],
    },
  },
  jobs: {
    lint_test: {
      docker: dockerCfgRustWithNodeImg,
      // We use arm instances for linting and testing because of
      // https://github.com/denoland/deno_core/issues/217
      resource_class: "arm.large",
      steps: [
        addSSHKeyStep,
        "checkout",
        addRestoreCacheStep,
        {
          run: {
            name: "cargo fmt check",
            command: "cargo fmt --check",
          },
        },
        {
          run: {
            name: "cargo build check",
            command: "cargo check",
          },
        },
        {
          run: {
            name: "install pnpm",
            command:
              "sudo corepack enable && sudo corepack prepare pnpm@latest-8 --activate",
          },
        },
        {
          run: {
            name: "install test script dependencies",
            working_directory: "./tests/fixtures",
            command: "pnpm install",
          },
        },
        {
          run: {
            name: "cargo test",
            command: "cargo test",
          },
        },
        addSaveCacheStep,
      ],
    },
    build_linux_amd64: getBuildUnixJob(
      executors.linux,
      "senc-linux-amd64.tar.gz",
    ),
    build_linux_arm64: getBuildUnixJob(
      executors.linuxarm,
      "senc-linux-arm64.tar.gz",
    ),
    build_darwin_arm64: getBuildUnixJob(
      executors.macos,
      "senc-darwin-arm64.tar.gz",
    ),
    build_windows_amd64: getBuildWindowsJob(),
    release: {
      docker: [
        {
          image: "cimg/node:lts",
        },
      ],
      steps: [
        addSSHKeyStep,
        {
          attach_workspace: {
            at: "/tmp/artifact",
          },
        },
        {
          run: {
            command: "ls -lah /tmp/artifact",
          },
        },
        "checkout",
        {
          run: {
            name: "download github-app-token CLI",
            working_directory: "/tmp",
            command: `
curl -sLO https://github.com/fensak-io/github-app-token/releases/download/v0.0.1/github-app-token_linux_amd64.tar.gz
tar -xvf github-app-token_linux_amd64.tar.gz
`,
          },
        },
        {
          run: {
            name: "semantic-release",
            command: `
export GITHUB_APP_PRIVATE_KEY="$(echo -n "$GITHUB_APP_PRIVATE_KEY_B64" | base64 -d)"
export GITHUB_TOKEN="$(/tmp/github-app-token --repo fensak-io/senc)"

npm install semantic-release-replace-plugin @semantic-release/git
npx -y semantic-release@^22.0.5
`,
          },
        },
      ],
    },
  },
};

export function main(): senc.OutData {
  return new senc.OutData({
    out_path: ".circleci/config.yml",
    out_type: "yaml",
    out_prefix: outPrefix,
    data: cfg,
  });
}
