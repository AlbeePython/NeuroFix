import socket
import sys
from dns import message, query

# Настройки целевого DNS-сервера
TARGET_DNS_HOST = "xbox-dns.ru"
DNS_PORT = 53

# Локальный адрес для запуска прокси
LOCAL_HOST = "127.0.0.1"
LOCAL_PORT = 53  # Внимание: порт 53 может требовать прав администратора/root

def get_target_ip(hostname):
    """Получает актуальный IP-адрес хоста xbox-dns.ru"""
    try:
        return socket.gethostbyname(hostname)
    except socket.gaierror:
        print(f"Ошибка: Не удалось разрешить имя {hostname}. Проверьте подключение к интернету.")
        sys.exit(1)

def start_dns_proxy():
    target_ip = get_target_ip(TARGET_DNS_HOST)
    print(f"[*] Целевой IP для {TARGET_DNS_HOST}: {target_ip}")

    # Создаем UDP сокет для прослушивания локальных запросов
    server_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    
    try:
        server_socket.bind((LOCAL_HOST, LOCAL_PORT))
        print(f"[*] Локальный DNS-прокси запущен на {LOCAL_HOST}:{LOCAL_PORT}")
        print("[*] Перенаправление трафика на xbox-dns.ru активно...")
    except PermissionError:
        print(f"[-] Ошибка: Для запуска на порту {LOCAL_PORT} требуются права администратора (sudo / запуск от админа).")
        sys.exit(1)
    except Exception as e:
        print(f"[-] Ошибка при привязке порта: {e}")
        sys.exit(1)

    server_socket.close()

if __name__ == "__main__":
    start_dns_proxy()
