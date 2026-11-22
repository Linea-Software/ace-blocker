import fs from "node:fs";
import path from "node:path";

const projectRoot = path.resolve(import.meta.dir, "..");
const pathsToRemove = ["node_modules/.vite", "build"];

console.log("Clearing cache...");

for (const p of pathsToRemove) {
    const fullPath = path.join(projectRoot, p);
    if (fs.existsSync(fullPath)) {
        console.log(`Removing ${fullPath}`);
        fs.rmSync(fullPath, { recursive: true, force: true });
    }
}

console.log("Cache cleared successfully.");
