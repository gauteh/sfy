import { app, BrowserWindow } from "electron";

let window: BrowserWindow | null;

const createWindow = () => {
  window = new BrowserWindow({
    width: 800,
    height: 600,
    frame: false,
    webPreferences: {
      nodeIntegration: true // preferably get rid of this at some point
    }
  });

	window.loadFile ("index.html");

  window.on("closed", () => {
    window = null;
  });

  const webContents = window.webContents;

  const handleRedirect = (e, url) => {
    if(url != webContents.getURL()) {
      e.preventDefault()
      require('electron').shell.openExternal(url)
    }
  }

  webContents.on('will-navigate', handleRedirect)
  webContents.on('new-window', handleRedirect)
};

app.on("ready", createWindow);

app.on("window-all-closed", () => {
  // if (process.platform !== "darwin") {
    app.quit();
  // }
});

app.on("activate", () => {
  if (window === null) {
    createWindow();
  }
});

