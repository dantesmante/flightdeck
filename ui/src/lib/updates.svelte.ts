// Update state shared by the banner and Settings.

import { api, type UpdateInfo } from "./api";

export const updates = $state<{
  info: UpdateInfo | null;
  checking: boolean;
  installing: boolean;
  error: string | null;
  dismissed: boolean;
}>({ info: null, checking: false, installing: false, error: null, dismissed: false });

export async function checkForUpdates() {
  updates.checking = true;
  updates.error = null;
  try {
    updates.info = await api.checkUpdate();
  } catch (e) {
    updates.error = String(e);
  } finally {
    updates.checking = false;
  }
}

export async function installUpdate() {
  const url = updates.info?.download_url;
  if (!url) return;
  updates.installing = true;
  updates.error = null;
  try {
    await api.installUpdate(url);
  } catch (e) {
    updates.error = String(e);
    updates.installing = false;
  }
}
