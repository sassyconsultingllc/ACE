#!/usr/bin/env python3
"""
Sample Python Script - Sassy Browser Demo
Demonstrates syntax highlighting and code viewing
"""

import json
from dataclasses import dataclass
from typing import List, Optional

@dataclass
class Product:
    name: str
    price: float
    category: str
    in_stock: bool = True

class Inventory:
    def __init__(self):
        self.products: List[Product] = []
    
    def add_product(self, product: Product) -> None:
        self.products.append(product)
        print(f"Added: {product.name} - ${product.price:.2f}")
    
    def get_by_category(self, category: str) -> List[Product]:
        return [p for p in self.products if p.category == category]
    
    def total_value(self) -> float:
        return sum(p.price for p in self.products if p.in_stock)

if __name__ == "__main__":
    inv = Inventory()
    inv.add_product(Product("Laptop", 999.99, "Electronics"))
    inv.add_product(Product("Desk Chair", 299.50, "Furniture"))
    inv.add_product(Product("Monitor", 449.00, "Electronics"))
    
    print(f"\nTotal inventory value: ${inv.total_value():,.2f}")
