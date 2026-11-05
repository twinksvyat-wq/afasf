# Сетка

Таблица Excel (.xlsx) из фото или текста. Один интерфейс на все платформы: Windows, Android, iOS и браузер.

- **Фото**: камера, галерея, перетаскивание или вставка скриншота (Ctrl+V). Распознавание через Tesseract (русский + английский). Модуль загружается из интернета при первом запуске, дальше работает из кэша.
- **Текст**: строки из Excel, CSV, `;`, `|`, Markdown-таблицы, выравнивание пробелами. Разделитель определяется автоматически.
- Ячейки редактируются прямо в таблице. Можно добавлять и удалять строки и столбцы. Многострочная вставка раскладывается по ячейкам.
- Экспорт в .xlsx: шапка (по желанию), числа сохраняются числами (`1 200,50` → 1200.5, `15%` → 15 %). Телефоны и коды с ведущим нулём остаются текстом. Ширина столбцов подбирается автоматически, шапка закреплена, включён автофильтр.
- Без внешних библиотек для Excel: генератор xlsx встроен (`src/core.js`).

## Структура

```
src/core.js                 разбор текста, раскладка OCR по столбцам, генератор xlsx
src/index.template.html     интерфейс
build.js                    собирает index.html → dist/, desktop/, mobile/www/
desktop/                    Windows-приложение (Go): встроенный интерфейс + окно Edge/Chrome
tools/mksyso/               вшивает иконку в .exe
mobile/                     Android / iOS (Capacitor 7)
.github/workflows/build.yml автосборка exe, apk и ipa
```

## Сборка

Проще всего через GitHub: залейте папку в репозиторий. Вкладка **Actions** соберёт три файла: `Setka.exe`, `Setka.apk` и `Setka-unsigned.ipa`.

Вручную:

```bash
node build.js

# Windows (собирается с любой ОС, нужен Go 1.24+)
cd tools/mksyso && go run . ../../desktop/rsrc_windows_amd64.syso ../../assets/icon-{16,24,32,48,64,256}.png && cd ../..
cd desktop && GOOS=windows GOARCH=amd64 go build -ldflags "-s -w -H windowsgui" -o Setka.exe .

# Android (Node 22, JDK 21, Android SDK)
cd mobile && npm install && npm run android && npm run icons
cd android && ./gradlew assembleDebug        # app/build/outputs/apk/debug/app-debug.apk

# iOS (только macOS + Xcode)
cd mobile && npm install && npm run ios && npx cap open ios   # в Xcode выбрать команду подписи и нажать Run
```

### Примечания
- **Windows.** Exe не подписан, поэтому SmartScreen при первом запуске покажет «Подробнее → Выполнить в любом случае». Приложение открывает отдельное окно Edge без адресной строки и закрывается вместе с ним.
- **iOS.** Apple не разрешает ставить приложения без подписи. Неподписанный `.ipa` можно установить через Sideloadly или AltStore со своим Apple ID, либо подписать в Xcode. Без установки можно открыть веб-версию в Safari и нажать «Поделиться → На экран „Домой“».
- **Android.** Debug-APK ставится напрямую: нужно разрешить установку из неизвестных источников.
