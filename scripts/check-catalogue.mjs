// Le catalogue part tel quel chez l'utilisateur : validation stricte
import { readFileSync } from "node:fs";

const catalogue = JSON.parse(readFileSync(new URL("../dist/marketplace.json", import.meta.url)));
const fail = (message) => {
  console.error(`catalogue invalide : ${message}`);
  process.exit(1);
};

if (catalogue.schemaVersion !== 1) fail("schemaVersion doit valoir 1");
if (!Array.isArray(catalogue.models) || catalogue.models.length === 0) fail("aucun modèle");
if (!Array.isArray(catalogue.categories) || catalogue.categories.length === 0) fail("aucune catégorie");

const required = [
  "id",
  "name",
  "brand",
  "description",
  "license",
  "releaseDate",
  "releaseYear",
  "capabilities",
  "variants",
];

for (const model of catalogue.models) {
  for (const key of required) {
    if (model[key] === undefined) fail(`${model.id ?? "?"} : champ « ${key} » manquant`);
  }
  if (model.variants.length === 0) fail(`${model.id} : aucune variante`);
  for (const variant of model.variants) {
    if (!/^https:\/\//.test(variant.tag ?? "")) fail(`${model.id} : source non https`);
    if (typeof variant.params !== "number") fail(`${model.id} : params doit être un nombre`);
    if (typeof variant.storageGb !== "number") fail(`${model.id} : storageGb doit être un nombre`);
  }
}

console.log(`catalogue valide : ${catalogue.models.length} modèles`);
