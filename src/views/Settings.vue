<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useSettingsStore } from "../stores/settings";
import { useLicenseStore } from "../stores/license";
import ThemeSwitch from "../components/ThemeSwitch.vue";

const settingsStore = useSettingsStore();
const licenseStore = useLicenseStore();
const activationCode = ref("");
const activating = ref(false);
const message = ref("");

onMounted(async () => {
  await settingsStore.load();
  await licenseStore.check();
});

async function saveSettings() {
  try {
    await settingsStore.save();
    message.value = "设置已保存";
  } catch (err) {
    message.value = typeof err === "string" ? err : err instanceof Error ? err.message : "保存失败";
  }
}

async function activate() {
  activating.value = true;
  try {
    const ok = await licenseStore.activate(activationCode.value);
    message.value = ok ? "Pro 激活成功" : "激活码无效";
    if (ok) activationCode.value = "";
  } finally {
    activating.value = false;
  }
}
</script>

<template>
  <div class="settings-page">
    <header class="page-header">
      <h2>设置</h2>
      <n-button text @click="$router.push('/')">返回</n-button>
    </header>

    <n-form label-placement="left" label-width="120">
      <n-form-item label="最大记录数">
        <n-input-number
          v-model:value="settingsStore.settings.maxCount"
          :min="50"
          :max="5000"
          :step="50"
        />
      </n-form-item>

      <n-form-item label="全局快捷键">
        <n-input v-model:value="settingsStore.settings.hotkey" placeholder="Ctrl+Shift+V" />
      </n-form-item>

      <n-form-item label="保存时长">
        <n-select
          v-model:value="settingsStore.settings.retentionDays"
          :options="[
            { label: '7 天', value: 7 },
            { label: '30 天', value: 30 },
            { label: '90 天', value: 90 },
            { label: '永久', value: 0 },
          ]"
        />
      </n-form-item>

      <n-form-item label="主题">
        <ThemeSwitch />
      </n-form-item>

      <n-form-item label="开机自启">
        <n-switch v-model:value="settingsStore.settings.autostart" />
      </n-form-item>

      <n-form-item label="忽略程序">
        <n-dynamic-tags v-model:value="settingsStore.settings.ignoredApps" />
      </n-form-item>

      <n-divider />

      <n-form-item label="设备指纹">
        <n-input :value="licenseStore.deviceFingerprint" readonly />
      </n-form-item>

      <n-form-item label="Pro 激活">
        <n-space>
          <n-input
            v-model:value="activationCode"
            placeholder="输入激活码"
            :disabled="licenseStore.isPro"
          />
          <n-button
            type="primary"
            :loading="activating"
            :disabled="licenseStore.isPro"
            @click="activate"
          >
            {{ licenseStore.isPro ? "已激活" : "激活" }}
          </n-button>
        </n-space>
      </n-form-item>

      <n-form-item>
        <n-button type="primary" @click="saveSettings">保存设置</n-button>
      </n-form-item>

      <n-alert v-if="licenseStore.isPro" type="success" :bordered="false" class="pro-box">
        <template #header>Pro 已授权</template>
        <n-space>
          <n-tag type="info">加密存储接口</n-tag>
          <n-tag type="info">快捷短语模板接口</n-tag>
          <n-tag type="info">导出导入接口</n-tag>
          <n-tag type="info">纯文本粘贴接口</n-tag>
        </n-space>
      </n-alert>

      <p v-if="message" class="message">{{ message }}</p>
    </n-form>
  </div>
</template>

<style scoped>
.settings-page {
  padding: 12px;
  height: 100vh;
  overflow-y: auto;
  box-sizing: border-box;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.page-header h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.message {
  font-size: 13px;
  color: var(--text-secondary);
}

.pro-box {
  margin-bottom: 12px;
}
</style>
