import { greeting } from "./greeting.js";

// Taxus compiles this file and greeting.ts to browser-native JavaScript modules.
document.body.dataset.taxusGreeting = greeting;
