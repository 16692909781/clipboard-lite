import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export const useLicenseStore = defineStore("license", () => {
  const isPro = ref(false);
  const deviceFingerprint = ref("");

  async function check() {
    isPro.value = await invoke<boolean>("is_pro_licensed");
    deviceFingerprint.value = await invoke<string>("get_device_fingerprint");
  }

  async function activate(code: string) {
    const ok = await invoke<boolean>("activate_license", { code });
    if (ok) isPro.value = true;
    return ok;
  }

  return { isPro, deviceFingerprint, check, activate };
});
