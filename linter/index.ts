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

async function main() {
  const dir = path.join(__dirname, "../proposals")

  const files = fs.readdirSync(dir).map((f) => {
    return path.join(dir, f)
  })

  const linted = markdownlint.sync({
    files: files,
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
}

main()
  .then(() => {
    console.log("Finished Successfully")
    process.exit(0)
  })
  .catch((error) => {
    console.log(error)
    process.exit(1)
  })
