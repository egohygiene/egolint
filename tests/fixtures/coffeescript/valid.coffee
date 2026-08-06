class GreetingService
  constructor: (@prefix) ->

  greet: (name) ->
    "#{@prefix}, #{name}!"

service = new GreetingService "Hello"
service.greet "Ego Hygiene"
