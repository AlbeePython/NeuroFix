#include <iostream>
#include <cstdlib>
#include <string>

// Функция для выполнения команды в консоли и проверки результата
void executeCommand(const std::string& command) {
    int result = std::system(command.c_str());
    if (result == 0) {
        std::cout << "[УСПЕШНО] Команда выполнена: " << command << std::endl;
    } else {
        std::cerr << "[ОШИБКА] Не удалось выполнить: " << command << std::endl;
        std::cerr << "Убедитесь, что программа запущена ОТ ИМЕНИ АДМИНИСТРАТОРА." << std::endl;
    }
}

int main() {
    // Настройка локали для корректного отображения русского языка в консоли
    std::setlocale(LC_ALL, "Russian");

    std::cout << "=== Изменение системного DNS ===" << std::endl;

    // Вставьте сюда реальные IP-адреса от xbox-dns.ru
    std::string primaryDNS = "1.1.1.1";   // Предположим, это основной DNS
    std::string secondaryDNS = "8.8.8.8"; // Предположим, это альтернативный DNS

    // Имя сетевого интерфейса. В русской Windows по умолчанию используется "Ethernet" или "Беспроводная сеть"
    // Скрипт ниже использует универсальный подход через PowerShell, чтобы применить настройки ко ВСЕМ активным адаптерам
    
    std::cout << "Настройка основного DNS (" << primaryDNS << ")..." << std::endl;
    std::string cmdPrimary = "powershell -Command \"Set-DnsClientServerAddress -InterfaceAlias (Get-NetAdapter | Where-Object {$_.Status -eq 'Up'}).Name -ServerAddresses (" + primaryDNS + "," + secondaryDNS + ")\"";
    
    // Выполняем команду
    executeCommand(cmdPrimary);

    // Сброс кэша DNS, чтобы изменения вступили в силу немедленно
    std::cout << "\nСброс кэша DNS (ipconfig /flushdns)..." << std::endl;
    executeCommand("ipconfig /flushdns");

    std::cout << "\nПроцесс завершен. Проверьте состояние сети." << std::endl;
    
    std::system("pause");
    return 0;
}
