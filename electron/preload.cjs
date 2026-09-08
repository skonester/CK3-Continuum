'use strict';
const { contextBridge, ipcRenderer } = require('electron');
contextBridge.exposeInMainWorld('continuum', {
  openSave: (slot) => ipcRenderer.invoke('save:open', slot),
  cancelInspection: () => ipcRenderer.invoke('save:cancel'),
  exportReport: () => ipcRenderer.invoke('report:export'),
  convertSave: (mode) => ipcRenderer.invoke('save:convert', mode)
});
