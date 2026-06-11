@echo off
chcp 65001 > nul
title Работа с API xbox-dns.ru
cls

:: --- НАСТРОЙКИ ---
set "API_URL=https://api.xbox-dns.ru"
:: Укажите ваш персональный токен или ключ API, если он необходим
set "API_TOKEN=ВАШ_ТОКЕН_ЗДЕСЬ"
:: -----------------

:MENU
cls
echo ===================================================
echo           УПРАВЛЕНИЕ XBOX-DNS.RU VIA API           
echo ===================================================
echo [1] Проверить статус сервиса / текущий IP
echo [2] Привязать текущий IP-адрес к аккаунту
echo [3] Обновить DNS-записи (DDNS)
echo [4] Показать информацию о подписке
echo [5] Выход
echo ===================================================
echo.

set /p choice="Выберите пункт меню (1-5): "

if "%choice%"=="1" goto STATUS
if "%choice%"=="2" goto BIND_IP
if "%choice%"=="3" goto UPDATE_DDNS
if "%choice%"=="4" goto INFO
if "%choice%"=="5" goto EXIT

echo Неверный выбор, попробуйте снова.
timeout /t 2 > nul
goto MENU

:STATUS
cls
echo [!] Запрос статуса сервера...
:: Пример GET-запроса
curl -s -X GET "%API_URL%/status" -H "Authorization: Bearer %API_TOKEN%"
echo.
goto PAUSE_AND_MENU

:BIND_IP
cls
echo [!] Отправка запроса на привязку вашего текущего IP...
:: Пример POST-запроса без тела (авто-апдейт по IP отправителя)
curl -s -X POST "%API_URL%/bind" -H "Authorization: Bearer %API_TOKEN%"
echo.
goto PAUSE_AND_MENU

:UPDATE_DDNS
cls
echo [!] Обновление DDNS...
:: Пример POST-запроса с отправкой JSON-данных
curl -s -X POST "%API_URL%/update" ^
     -H "Content-Type: application/json" ^
     -H "Authorization: Bearer %API_TOKEN%" ^
     -d "{\"action\":\"refresh\"}"
echo.
goto PAUSE_AND_MENU

:INFO
cls
echo [!] Получение информации о профиле...
curl -s -X GET "%API_URL%/user/profile" -H "Authorization: Bearer %API_TOKEN%"
echo.
goto PAUSE_AND_MENU

:PAUSE_AND_MENU
echo.
echo ===================================================
echo Нажмите любую клавишу для возврата в меню...
pause > nul
goto MENU

:EXIT
cls
echo До свидания!
timeout /t 2 > nul
exit
