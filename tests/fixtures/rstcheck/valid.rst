rstcheck Integration Fixture
============================

This document validates repository reStructuredText conventions.

Capabilities
------------

* reStructuredText syntax validation
* Sphinx directive compatibility
* Embedded Python validation

Example
-------

The following function returns a greeting:

.. code-block:: python

   def format_greeting(name: str) -> str:
       """Return a greeting for the supplied name."""
       normalized_name = name.strip()

       if not normalized_name:
           return "Hello."

       return f"Hello, {normalized_name}."

Sphinx integration
------------------

.. automodule:: egohygiene.example
   :members:

See :doc:`architecture` for additional information.

