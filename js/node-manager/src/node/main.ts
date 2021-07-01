/* eslint-disable @typescript-eslint/ban-ts-comment */

import { App, app, BrowserWindow } from 'electron'
import { CONFIG } from '../config'
import { setupIpc } from './ipc'
import { format as formatUrl } from 'url'
import * as path from 'path'

console.log(`NODE_ENV: ${process.env.NODE_ENV}`)
const isDevelopment = process.env.NODE_ENV !== 'production'

const createWindow = (app: App): BrowserWindow => {
  const mainWindow = new BrowserWindow({
    width: CONFIG.windowSize.initial.width,
    height: CONFIG.windowSize.initial.height,
    minWidth: CONFIG.windowSize.minimum.width,
    minHeight: CONFIG.windowSize.minimum.height,
    show: false,
    webPreferences: {
      nodeIntegration: true,
      // FIXME: https://www.electronjs.org/docs/tutorial/context-isolation
      contextIsolation: false,
      devTools: true,
      enableRemoteModule: true, // ONLY FOR TESTING
    },
    autoHideMenuBar: true,
  })

  if (isDevelopment) {
    mainWindow.loadURL(`http://localhost:${process.env.ELECTRON_WEBPACK_WDS_PORT}`)
  } else {
    mainWindow.loadURL(
      formatUrl({
        pathname: path.join(__dirname, 'index.html'),
        protocol: 'file',
        slashes: true,
      }),
    )
  }

  mainWindow.webContents.on('did-finish-load', function () {
    mainWindow.show()
  })

  mainWindow.webContents.on('new-window', function (e, url) {
    e.preventDefault()
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    require('electron').shell.openExternal(url)
  })

  setupIpc(app, mainWindow)

  // Open the DevTools.
  // mainWindow.webContents.openDevTools()
  return mainWindow
}

const start = () => {
  createWindow(app)
}

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.on('ready', () => {
  start()
})

// Quit when all windows are closed.
app.on('window-all-closed', () => {
  app.quit()
})

app.on('activate', () => {
  // On OS X it's common to re-create a window in the app when the
  // dock icon is clicked and there are no other windows open.
  if (BrowserWindow.getAllWindows().length === 0) {
    start()
  }
})
