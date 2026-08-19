#include <iostream>
#include <string>
#include <vector>

namespace fixtures
{

class GreetingService
{
public:
  explicit GreetingService(std::string prefix) : prefix_(std::move(prefix)) {}

  [[nodiscard]] std::string create_greeting(const std::string& name) const
  {
    if (name.empty())
    {
      return prefix_;
    }

    return prefix_ + ", " + name + "!";
  }

private:
  std::string prefix_;
};

}  // namespace fixtures

int main()
{
  const fixtures::GreetingService greeting_service("Hello");
  const std::vector<std::string> names{
      "C",
      "C++",
      "Ego Hygiene",
  };

  for (const std::string& name : names)
  {
    std::cout << greeting_service.create_greeting(name) << '\n';
  }

  return 0;
}
