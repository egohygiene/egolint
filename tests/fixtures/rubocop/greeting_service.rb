# frozen_string_literal: true

module EgoHygiene
  module Fixtures
    class GreetingService
      def format_greeting(name)
        normalized_name = name.strip

        return 'Hello.' if normalized_name.empty?

        "Hello, #{normalized_name}."
      end
    end
  end
end
