<?php

declare(strict_types=1);

namespace EgoHygiene\Tests\Fixtures\Php;

final class ValidGreetingService
{
    public function formatGreeting(string $name): string
    {
        $normalizedName = trim($name);

        if ($normalizedName === '') {
            return 'Hello.';
        }

        return sprintf('Hello, %s.', $normalizedName);
    }
}
