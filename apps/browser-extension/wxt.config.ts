import { defineConfig } from "wxt";

export default defineConfig({
  modules: ["@wxt-dev/module-react"],
  srcDir: "src",
  manifest: {
    name: "TF0000",
    description: "Capture and carry local context across ChatGPT, Claude, and Gemini.",
    version: "0.7.0",
    permissions: ["activeTab", "clipboardWrite", "nativeMessaging", "sidePanel", "storage"],
    host_permissions: [
      "https://chatgpt.com/*",
      "https://claude.ai/*",
      "https://gemini.google.com/*",
    ],
    action: {
      default_title: "Open TF0000",
    },
    side_panel: {
      default_path: "sidepanel.html",
    },
  },
});
