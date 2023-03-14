import markdownlint from "markdownlint"
import {
  enforceHeaderStructure,
  enforceMetadataStructure,
  metadataSimdIsValid,
  metadataTitleIsValid,
  metadataAuthorsIsValid,
  metadataCategoryIsValid,
  metadataTypeIsValid,
  metadataStatusIsValid,
} from "./customRules"
import fs from "fs"
import path from "path"
const core = require('@actions/core');


async function main() {
  const dir = path.join(__dirname, "../../proposals")

  const files = fs.readdirSync(dir).filter((f) => {
    if (f.indexOf("0001-simd-process.md")) {
      return true
    }
    return false
  }).map((f) => {
    return path.join(dir, f)
  })

  //const configuration = markdownlint.readConfigSync('../config/.markdownlint.json')
  
  const linted = markdownlint.sync({
    files: files,
    config: {
      default: true,
      MD001: true,
      MD002: false,
      MD003: {
        style: "atx"
      },
      MD004: {
        style: "consistent"
      },
      MD005: true,
      MD006: false,
      MD007: false,
      MD009: false,
      MD010: false,
      MD011: true,
      MD012: false,
      MD013: true,
      MD014: false,
      MD018: true,
      MD019: true,
      MD020: true,
      MD021: true,
      MD022: true,
      MD023: true,
      MD024: {
        allow_different_nesting: true
      },
      MD025: {
        level: 1,
        front_matter_title: "^\\s*title\\s*[:=]"
      },
      MD026: false,
      MD027: false,
      MD028: true,
      MD029: false,
      MD030: {
        ul_single: 1,
        ol_single: 1,
        ul_multi: 1,
        ol_multi: 1
      },
      MD031: {
        list_items: true
      },
      MD032: true,
      MD033: {
        allowed_elements: []
      },
      MD034: false,
      MD035: {
        style: "consistent"
      },
      MD036: false,
      MD037: false,
      MD038: true,
      MD039: false,
      MD040: false,
      MD041: {
        level: 1,
        front_matter_title: "^\\s*title\\s*[:=]"
      },
      MD042: true,
      MD043: false,
      MD044: {
        names: [],
        code_blocks: true,
        html_elements: true
      },
      MD045: false,
      MD046: {
        style: "consistent"
      },
      MD047: false,
      MD048: {
        style: "consistent"
      },
      MD049: {
        style: "consistent"
      },
      MD050: {
        style: "consistent"
      },
      MD051: true,
      MD052: true,
      MD053: true
    },
    customRules: [
      enforceHeaderStructure,
      enforceMetadataStructure,
      metadataSimdIsValid,
      metadataTitleIsValid,
      metadataAuthorsIsValid,
      metadataCategoryIsValid,
      metadataTypeIsValid,
      metadataStatusIsValid,
    ],
  })
  console.log(linted)
  let errorCount = 0
  for (let lint in linted) {
    errorCount += linted[lint].length
  }
  if (errorCount > 0) {
    throw new Error(JSON.stringify(linted))
  }
}

main()
  .then(() => {
    console.log("Finished Successfully")
    process.exit(0)
  })
  .catch((error) => {
    core.setFailed(error)
    process.exit(1)
  })