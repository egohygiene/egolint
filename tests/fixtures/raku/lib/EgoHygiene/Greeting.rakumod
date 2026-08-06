use v6.d;

unit module EgoHygiene::Greeting;

sub format-greeting(Str:D $name --> Str:D) is export {
    my Str:D $normalized-name = $name.trim;

    return 'Hello.' if $normalized-name eq '';

    "Hello, {$normalized-name}."
}

