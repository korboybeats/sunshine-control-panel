import { Menu, dialog, shell, app } from 'electron'
import openAboutWindow from 'about-window'
import { fileURLToPath } from 'node:url'
import { join, dirname, parse } from 'node:path'
import { createSubBrowserWin, runCmdAsAdmin } from './utils.js'
import { SUNSHINE_PATH, SUNSHINE_TOOLS_PATH, VIRTUAL_DRIVER_PATH } from './paths.js'
import sudo from 'sudo-prompt'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

export function createMenuTemplate(mainWindow) {
  const isMac = process.platform === 'darwin'

  return [
    // { role: 'appMenu' }
    ...(isMac
      ? [
          {
            label: app.name,
            submenu: [
              { role: 'about' },
              { type: 'separator' },
              { role: 'services' },
              { type: 'separator' },
              { role: 'hide' },
              { role: 'hideOthers' },
              { role: 'unhide' },
              { type: 'separator' },
              { role: 'quit' },
            ],
          },
        ]
      : []),
    {
      label: 'Window',
      submenu: [
        { role: 'minimize' },
        { role: 'zoom' },
        { role: 'reload' },
        ...(isMac
          ? [{ type: 'separator' }, { role: 'front' }, { type: 'separator' }, { role: 'window' }]
          : [{ role: 'close' }]),
      ],
    },
    {
      label: 'Manage',
      submenu: [
        {
          label: 'Edit Virtual Display Resolution',
          click: () => {
            const subWin = createSubBrowserWin(null, mainWindow)
            subWin.loadFile(join(__dirname, '../renderer/vdd/index.html'))
          },
        },
        {
          label: 'Uninstall Virtual Display',
          click: async () => {
            const prompt = await dialog.showMessageBox(mainWindow, {
              type: 'question',
              message: 'Are you sure you want to uninstall? You can restore it by reinstalling the base version of Sunshine.',
              buttons: ['Cancel', 'Confirm'],
            })
            if (prompt.response) {
              const uninstallCmd = [
                `"${join(VIRTUAL_DRIVER_PATH, 'nefconw.exe')}"`,
                '--remove-device-node',
                '--hardware-id ROOT\\iddsampledriver',
                '--class-guid 4d36e968-e325-11ce-bfc1-08002be10318',
              ].join(' ')

              runCmdAsAdmin(uninstallCmd).on('close', (code) => {
                dialog.showMessageBox(mainWindow, {
                  message: `Uninstallation of virtual display completed: ${code}`,
                })
              })
            }
          },
        },
        { type: 'separator' },
        {
          label: 'Restart Graphics Driver',
          click: () => {
            const restartExe = join(SUNSHINE_TOOLS_PATH, 'restart64.exe')
            sudo.exec(`"${restartExe}"`, {
              name: 'Sunshine Control Panel',
            })
          },
        },
        {
          label: 'Restart Sunshine as Administrator',
          click: () => {
            const command = [
              'net stop sunshineservice',
              'taskkill /IM sunshine.exe /F',
              `cd "${SUNSHINE_PATH}"`,
              './sunshine.exe',
            ].join(' && ')

            runCmdAsAdmin(command).on('close', () => mainWindow.close())
          },
        },
      ],
    },
    {
      label: 'User Guide',
      submenu: [
        {
          label: 'Download Latest Base Version of Sunshine',
          click: async () => {
            await shell.openExternal('https://github.com/qiin2333/Sunshine/releases/tag/alpha')
          },
        },
        {
          label: 'Join My QQ Group',
          click: async () => {
            const subWin = createSubBrowserWin(null, mainWindow)
            subWin.loadURL('https://qm.qq.com/q/s3QnqbxvFK')
            setTimeout(() => {
              subWin.close()
            }, 3000)
          },
        },
        {
          label: 'User Guide',
          click: async () => {
            await shell.openExternal('https://docs.qq.com/aio/DSGdQc3htbFJjSFdO')
          },
        },
      ],
    },
    {
      label: 'Tools',
      submenu: [
        {
          label: 'Clipboard Sync',
          click: async () => {
            const subWin = createSubBrowserWin(null, mainWindow)
            subWin.loadURL('https://gcopy.rutron.net/zh')
          },
        },
        {
          label: 'Timer for Streaming Screen Capture',
          click: () => {
            const subWin = createSubBrowserWin({ width: 1080, height: 600 }, mainWindow)
            subWin.loadFile(join(__dirname, '../renderer/stop-clock-canvas/index.html'))
          },
        },
        {
          label: 'New Generation Delay Test Clock by Kile',
          click: async () => {
            const subWin = createSubBrowserWin(null, mainWindow)
            subWin.loadURL('https://yangkile.github.io/D-lay/')
          },
        },
        {
          label: 'Gamepad Testing',
          click: async () => {
            await shell.openExternal('https://hardwaretester.com/gamepad')
          },
        },
      ],
    },
    {
      label: 'About',
      click: () =>
        openAboutWindow.default({
          icon_path: 'https://raw.gitmirror.com/qiin2333/qiin.github.io/assets/img/109527119_p1.png',
          product_name: 'Sunshine Base Version',
          copyright: 'Copyright (c) 2023 Qiin',
          use_version_info: false,
          package_json_dir: __dirname,
        }),
    },
  ]
}

export function setupApplicationMenu(mainWindow) {
  const menu = Menu.buildFromTemplate(createMenuTemplate(mainWindow))
  Menu.setApplicationMenu(menu)
}
