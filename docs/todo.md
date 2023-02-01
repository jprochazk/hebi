- classes + method call + bound methods
- indexing ops split out from the VM (might become a trait, should be used for making it easier to deal with dict[key] vs list[key] vs class[key] etc.)
- iterator protocol + range-based loop using iterators (for v in obj) (blocked by meta methods)
- yield (and generators)
- modules and import
- native functions/classes/modules and a nice API for creating and exposing them to Mu-land 
- meta methods

### Classes (notes)

```python
class T:
  v = 0

  init(self, u):
    if u:
      self.v = 10
      return

T(true)  # T { v: 10 }
T(false) # T { v: 0 }


class V(T):
  pass


```


```
CreateClass:
1. Create empty class
2. Eval field defaults + store in class
3. Init methods + store in class
4. Freeze class
5. Store in accumulator

CreateClassDerived:
1. Load parent
2. Create class from parent
3. Eval field defaults + store in class
4. Init methods + store in class
5. Freeze class
6. Store in accumulator


```
