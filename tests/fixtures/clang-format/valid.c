#include <stdbool.h>
#include <stdio.h>
#include <string.h>

typedef struct GreetingService
{
  const char* prefix;
} GreetingService;

static bool is_empty(const char* value)
{
  return value == NULL || strlen(value) == 0;
}

static void print_greeting(const GreetingService* service, const char* name)
{
  if (service == NULL || is_empty(service->prefix))
  {
    return;
  }

  if (is_empty(name))
  {
    printf("%s\n", service->prefix);
    return;
  }

  printf("%s, %s!\n", service->prefix, name);
}

int main(void)
{
  const GreetingService greeting_service = {
      .prefix = "Hello",
  };

  const char* names[] = {
      "C",
      "Ego Hygiene",
  };

  const size_t name_count = sizeof(names) / sizeof(names[0]);

  for (size_t index = 0; index < name_count; ++index)
  {
    print_greeting(&greeting_service, names[index]);
  }

  return 0;
}

