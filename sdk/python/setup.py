#!/usr/bin/env python3
"""Setup script for CAGE Python SDK"""

from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="cage-sdk",
    version="1.0.0",
    author="CAGE Project",
    author_email="",
    description="Python SDK for CAGE - Contained AI-Generated Code Execution",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/cage-project/cage",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 5 - Production/Stable",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
    ],
    python_requires=">=3.8",
    install_requires=[
        "requests>=2.28.0",
        "websockets>=11.0",
    ],
    extras_require={
        "dev": ["pytest>=7.0", "black", "mypy", "flake8"],
    },
)
